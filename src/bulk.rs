//! Bulk generation and string materialization.
//!
//! The single-ID path in the crate root remains allocation-light and frozen.
//! These APIs add scoped supervisor workers, exact 150-bit OS extraction, TPM
//! 64-byte reads, streaming consumers, and contiguous encoded batches.

use std::time::SystemTime;

use crate::id::{EncodedBatch, RailId, ENCODED_LEN, RANDOM_ACQUIRE_BYTES};
use crate::os_random::OsBitReader;
use crate::rand::combine_hybrid;
use crate::{
    generate_os_only, generate_with_source, ticks_from_system_time, Profile, RailIdError,
    RandomSource, SystemRandomSource, BULK_MIN_IDS_PER_WORKER, TPM_BULK_READ_BYTES,
};

fn reserve_vec<T>(count: usize) -> Result<Vec<T>, RailIdError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| RailIdError::CapacityOverflow)?;
    Ok(values)
}

fn zeroed_vec(count: usize) -> Result<Vec<u8>, RailIdError> {
    let mut values = reserve_vec(count)?;
    values.resize(count, 0);
    Ok(values)
}

/// Recommended number of OS-only workers for a bulk request.
///
/// Returns `0` for `count == 0`, and otherwise caps workers so each chunk is
/// around [`BULK_MIN_IDS_PER_WORKER`] IDs. Small batches return `1` and stay
/// sequential; a tail chunk may be slightly smaller.
#[must_use]
pub fn bulk_worker_count(count: usize) -> usize {
    if count == 0 {
        return 0;
    }

    let cpus = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1)
        .max(1);
    let by_work = (count / BULK_MIN_IDS_PER_WORKER).max(1);
    cpus.min(by_work).max(1)
}

/// Generate `count` RailIDs in one call.
///
/// The system is used as follows:
///
/// * [`Profile::OsOnlyDegraded`]: the calling thread supervises a scoped worker
///   group, splits the request into contiguous chunks, and generates the chunks
///   in parallel. Worker threads are created for this call and are not pooled.
/// * [`Profile::TpmHybrid`]: TPM bytes are read in [`TPM_BULK_READ_BYTES`]
///   chunks and sliced into fresh 19-byte chunks. The TPM is a single device,
///   so this path is serialized; the larger read reduces TPM command count.
///
/// The returned vector preserves chunk order, but parallel OS-only generation
/// does not guarantee globally chronological order across chunks. Sort by the
/// `RailId` value if timestamp-first ordering is required.
///
/// # Errors
///
/// Propagates timestamp, TPM, or OS randomness failures. No partial result is
/// returned; either all IDs are generated or the call fails.
pub fn generate_many(count: usize, profile: Profile) -> Result<Vec<RailId>, RailIdError> {
    match profile {
        Profile::OsOnlyDegraded => parallel_os_chunks(count, generate_os_chunk),
        Profile::TpmHybrid => {
            let mut source = SystemRandomSource;
            generate_many_tpm(count, &mut source)
        }
    }
}

/// Generate `count` RailIDs and encode each as unpadded Base64URL.
///
/// This is equivalent to [`generate_many`] followed by encoding, but the
/// OS-only path performs encoding inside its parallel workers.
///
/// # Errors
///
/// See [`generate_many`].
pub fn generate_many_strings(count: usize, profile: Profile) -> Result<Vec<String>, RailIdError> {
    match profile {
        Profile::OsOnlyDegraded => parallel_os_chunks(count, generate_os_strings_chunk),
        Profile::TpmHybrid => {
            let mut source = SystemRandomSource;
            let mut strings = reserve_vec(count)?;
            for_each_tpm(count, &mut source, |id| {
                strings.push(id.encode());
                Ok(())
            })?;
            Ok(strings)
        }
    }
}

/// Generate `count` strings and deliver each one to `consume` inside the
/// generation workers.
///
/// Unlike [`generate_many_strings`], this does not materialize a
/// `Vec<String>` and does not move string ownership back to the caller.
/// OS-only workers invoke `consume` concurrently and in no guaranteed order;
/// TPM generation invokes it serially. A consumer that only hashes or forwards
/// strings can therefore keep both allocation and deallocation inside the
/// worker thread. The consumer must be `Send + Sync`; use atomics or per-call
/// synchronization for shared state.
///
/// # Errors
///
/// Propagates timestamp or randomness failures. If a worker panics, the panic
/// is resumed in the caller.
pub fn for_each_many_strings<F>(
    count: usize,
    profile: Profile,
    consume: F,
) -> Result<(), RailIdError>
where
    F: Fn(String) + Send + Sync,
{
    if count == 0 {
        return Ok(());
    }

    match profile {
        Profile::OsOnlyDegraded => {
            parallel_os_for_each(count, |len| generate_os_strings_chunk_into(len, &consume))
        }
        Profile::TpmHybrid => {
            let mut source = SystemRandomSource;
            for_each_tpm(count, &mut source, |id| {
                consume(id.encode());
                Ok(())
            })
        }
    }
}

/// Generate `count` IDs and deliver each encoded string as a borrowed `&str`
/// inside the generation workers.
///
/// This is the allocation-free counterpart of [`for_each_many_strings`]: the
/// encoding uses a stack-allocated fixed-size buffer, and the consumer borrows
/// it only for the duration of the callback. It cannot retain the reference.
/// OS-only consumers are invoked concurrently and in no guaranteed order; TPM
/// consumers are invoked serially.
///
/// # Errors
///
/// Propagates timestamp or randomness failures. If a worker panics, the panic
/// is resumed in the caller.
pub fn for_each_many_encoded<F>(
    count: usize,
    profile: Profile,
    consume: F,
) -> Result<(), RailIdError>
where
    F: Fn(&str) + Send + Sync,
{
    if count == 0 {
        return Ok(());
    }

    match profile {
        Profile::OsOnlyDegraded => {
            parallel_os_for_each(count, |len| generate_os_encoded_chunk_into(len, &consume))
        }
        Profile::TpmHybrid => {
            let mut source = SystemRandomSource;
            for_each_tpm(count, &mut source, |id| {
                let encoded = id.encode_array();
                consume(encoded.as_str());
                Ok(())
            })
        }
    }
}

/// Generate `count` IDs into one contiguous Base64URL batch.
///
/// The result owns a single `Vec<u8>` of `count * 32` bytes instead of one
/// `String` per ID. Entries are accessed with [`EncodedBatch::get`] or
/// [`EncodedBatch::iter`]. OS-only generation writes disjoint per-worker
/// segments in parallel; TPM generation remains serialized.
///
/// # Errors
///
/// Returns [`RailIdError::CapacityOverflow`] if `count * 32` overflows, and
/// propagates generation failures.
pub fn generate_many_encoded_batch(
    count: usize,
    profile: Profile,
) -> Result<EncodedBatch, RailIdError> {
    if count == 0 {
        return Ok(EncodedBatch::from_data(Vec::new()));
    }

    let total_bytes = count
        .checked_mul(ENCODED_LEN)
        .ok_or(RailIdError::CapacityOverflow)?;
    let mut data = zeroed_vec(total_bytes)?;

    match profile {
        Profile::OsOnlyDegraded => fill_os_encoded_batch(count, &mut data)?,
        Profile::TpmHybrid => {
            let mut source = SystemRandomSource;
            let mut index = 0usize;
            for_each_tpm(count, &mut source, |id| {
                let encoded = id.encode_array();
                let start = index * ENCODED_LEN;
                data[start..start + ENCODED_LEN].copy_from_slice(encoded.as_bytes());
                index += 1;
                Ok(())
            })?;
        }
    }

    Ok(EncodedBatch::from_data(data))
}

/// Generate `count` RailIDs with a caller-supplied entropy source.
///
/// The TPM bulk path is exercised in both profiles. The OS-only path is
/// sequential because [`RandomSource`] is borrowed mutably and cannot be shared
/// between workers. Use [`generate_many`] for the parallel system path.
///
/// # Errors
///
/// Propagates timestamp or random source failures.
pub fn generate_many_with_source<S: RandomSource + ?Sized>(
    count: usize,
    profile: Profile,
    source: &mut S,
) -> Result<Vec<RailId>, RailIdError> {
    match profile {
        Profile::TpmHybrid => generate_many_tpm(count, source),
        Profile::OsOnlyDegraded => {
            let mut ids = reserve_vec(count)?;
            for _ in 0..count {
                let ticks = ticks_from_system_time(SystemTime::now())?;
                ids.push(generate_with_source(profile, ticks, source)?);
            }
            Ok(ids)
        }
    }
}

fn parallel_os_chunks<T, F>(count: usize, generate: F) -> Result<Vec<T>, RailIdError>
where
    T: Send,
    F: Fn(usize) -> Result<Vec<T>, RailIdError> + Sync,
{
    if count == 0 {
        return Ok(Vec::new());
    }

    let workers = bulk_worker_count(count);
    if workers <= 1 {
        return generate(count);
    }

    let chunk_size = count.div_ceil(workers);
    let mut all = reserve_vec(count)?;
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for worker in 0..workers {
            let start = worker.saturating_mul(chunk_size);
            let end = start.saturating_add(chunk_size).min(count);
            let length = end.saturating_sub(start);
            if length == 0 {
                continue;
            }
            let generate = &generate;
            handles.push(scope.spawn(move || -> Result<Vec<T>, RailIdError> { generate(length) }));
        }

        for handle in handles {
            match handle.join() {
                Ok(result) => all.extend(result?),
                Err(payload) => std::panic::resume_unwind(payload),
            }
        }
        Ok(all)
    })
}

fn generate_os_chunk(count: usize) -> Result<Vec<RailId>, RailIdError> {
    // Avoid a buffer allocation for tiny chunks.
    if count < 64 {
        return (0..count).map(|_| generate_os_only()).collect();
    }

    let mut reader = OsBitReader::new();
    let mut ids = reserve_vec(count)?;
    for _ in 0..count {
        let ticks = ticks_from_system_time(SystemTime::now())?;
        let random = reader.next_150()?;
        ids.push(RailId::from_parts(ticks, random)?);
    }
    Ok(ids)
}

fn generate_os_strings_chunk(count: usize) -> Result<Vec<String>, RailIdError> {
    if count < 64 {
        return (0..count)
            .map(|_| generate_os_only().map(|id| id.encode()))
            .collect();
    }

    let mut reader = OsBitReader::new();
    let mut strings = reserve_vec(count)?;
    for _ in 0..count {
        let ticks = ticks_from_system_time(SystemTime::now())?;
        let random = reader.next_150()?;
        let id = RailId::from_parts(ticks, random)?;
        strings.push(id.encode());
    }
    Ok(strings)
}

fn generate_os_strings_chunk_into<F>(count: usize, consume: &F) -> Result<(), RailIdError>
where
    F: Fn(String) + Sync,
{
    generate_os_encoded_chunk_into(count, &|encoded: &str| consume(encoded.to_owned()))
}

fn generate_os_encoded_chunk_into<F>(count: usize, consume: &F) -> Result<(), RailIdError>
where
    F: Fn(&str) + Sync,
{
    if count < 64 {
        for _ in 0..count {
            let encoded = generate_os_only()?.encode_array();
            consume(encoded.as_str());
        }
        return Ok(());
    }

    let mut reader = OsBitReader::new();
    for _ in 0..count {
        let ticks = ticks_from_system_time(SystemTime::now())?;
        let random = reader.next_150()?;
        let id = RailId::from_parts(ticks, random)?;
        let encoded = id.encode_array();
        consume(encoded.as_str());
    }
    Ok(())
}

fn fill_os_encoded_batch(count: usize, data: &mut [u8]) -> Result<(), RailIdError> {
    debug_assert_eq!(data.len(), count * ENCODED_LEN);

    let workers = bulk_worker_count(count);
    if workers <= 1 {
        return fill_os_encoded_segment(data);
    }

    let chunk_ids = count.div_ceil(workers);
    let chunk_bytes = chunk_ids
        .checked_mul(ENCODED_LEN)
        .ok_or(RailIdError::CapacityOverflow)?;
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for segment in data.chunks_mut(chunk_bytes) {
            handles.push(scope.spawn(move || fill_os_encoded_segment(segment)));
        }

        for handle in handles {
            match handle.join() {
                Ok(result) => result?,
                Err(payload) => std::panic::resume_unwind(payload),
            }
        }
        Ok(())
    })
}

fn fill_os_encoded_segment(segment: &mut [u8]) -> Result<(), RailIdError> {
    let count = segment.len() / ENCODED_LEN;
    if count < 64 {
        for index in 0..count {
            let encoded = generate_os_only()?.encode_array();
            let start = index * ENCODED_LEN;
            segment[start..start + ENCODED_LEN].copy_from_slice(encoded.as_bytes());
        }
        return Ok(());
    }

    let mut reader = OsBitReader::new();
    for index in 0..count {
        let ticks = ticks_from_system_time(SystemTime::now())?;
        let random = reader.next_150()?;
        let id = RailId::from_parts(ticks, random)?;
        let encoded = id.encode_array();
        let start = index * ENCODED_LEN;
        segment[start..start + ENCODED_LEN].copy_from_slice(encoded.as_bytes());
    }
    Ok(())
}

fn parallel_os_for_each<F>(count: usize, generate: F) -> Result<(), RailIdError>
where
    F: Fn(usize) -> Result<(), RailIdError> + Sync,
{
    if count == 0 {
        return Ok(());
    }

    let workers = bulk_worker_count(count);
    if workers <= 1 {
        return generate(count);
    }

    let chunk_size = count.div_ceil(workers);
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for worker in 0..workers {
            let start = worker.saturating_mul(chunk_size);
            let end = start.saturating_add(chunk_size).min(count);
            let length = end.saturating_sub(start);
            if length == 0 {
                continue;
            }
            let generate = &generate;
            handles.push(scope.spawn(move || generate(length)));
        }

        for handle in handles {
            match handle.join() {
                Ok(result) => result?,
                Err(payload) => std::panic::resume_unwind(payload),
            }
        }
        Ok(())
    })
}

fn generate_many_tpm<S: RandomSource + ?Sized>(
    count: usize,
    source: &mut S,
) -> Result<Vec<RailId>, RailIdError> {
    if count == 0 {
        return Ok(Vec::new());
    }

    let mut ids = reserve_vec(count)?;
    for_each_tpm(count, source, |id| {
        ids.push(id);
        Ok(())
    })?;
    Ok(ids)
}

fn for_each_tpm<S, F>(count: usize, source: &mut S, mut consume: F) -> Result<(), RailIdError>
where
    S: RandomSource + ?Sized,
    F: FnMut(RailId) -> Result<(), RailIdError>,
{
    if count == 0 {
        return Ok(());
    }

    // A single ID should not waste a 64-byte TPM read.
    if count == 1 {
        let ticks = ticks_from_system_time(SystemTime::now())?;
        let id = generate_with_source(Profile::TpmHybrid, ticks, source)?;
        return consume(id);
    }

    let mut stream = TpmByteStream {
        buffer: [0u8; TPM_BULK_READ_BYTES + RANDOM_ACQUIRE_BYTES],
        len: 0,
    };

    for _ in 0..count {
        let ticks = ticks_from_system_time(SystemTime::now())?;
        let tpm_random = stream.next(&mut *source)?;

        let mut os_random = [0u8; RANDOM_ACQUIRE_BYTES];
        source.fill_os(&mut os_random)?;

        let random = combine_hybrid(&tpm_random, &os_random);
        consume(RailId::from_parts(ticks, random)?)?;
    }

    Ok(())
}

/// A buffered reader over `TPM2_GetRandom` that retains leftover bytes between
/// 64-byte requests.
struct TpmByteStream {
    buffer: [u8; TPM_BULK_READ_BYTES + RANDOM_ACQUIRE_BYTES],
    len: usize,
}

impl TpmByteStream {
    fn next<S: RandomSource + ?Sized>(
        &mut self,
        source: &mut S,
    ) -> Result<[u8; RANDOM_ACQUIRE_BYTES], RailIdError> {
        if self.len < RANDOM_ACQUIRE_BYTES {
            self.refill(source)?;
        }

        let mut out = [0u8; RANDOM_ACQUIRE_BYTES];
        out.copy_from_slice(&self.buffer[..RANDOM_ACQUIRE_BYTES]);
        self.buffer.copy_within(RANDOM_ACQUIRE_BYTES..self.len, 0);
        self.len -= RANDOM_ACQUIRE_BYTES;
        Ok(out)
    }

    fn refill<S: RandomSource + ?Sized>(&mut self, source: &mut S) -> Result<(), RailIdError> {
        let mut chunk = [0u8; TPM_BULK_READ_BYTES];
        source.fill_tpm(&mut chunk)?;

        let start = self.len;
        let end = start + TPM_BULK_READ_BYTES;
        if end > self.buffer.len() {
            return Err(RailIdError::TpmRandomFailure);
        }
        self.buffer[start..end].copy_from_slice(&chunk);
        self.len = end;
        Ok(())
    }
}
