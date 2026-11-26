//! Conformance and behaviour tests for RailID v1.

use std::collections::HashSet;

use railid::{
    bulk_worker_count, decode, encode_array, for_each_many_encoded, for_each_many_strings,
    from_parts, generate_many, generate_many_encoded_batch, generate_many_strings,
    generate_many_with_source, generate_with_source, Profile, RailId, RailIdError, RandomSource,
    RANDOM_ACQUIRE_BYTES, TIMESTAMP_MAX, TPM_BULK_READ_BYTES,
};

const ZERO_ID: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

/// Deterministic entropy source used to exercise the generation paths.
struct ScriptedSource {
    tpm_bytes: [u8; RANDOM_ACQUIRE_BYTES],
    os_bytes: [u8; RANDOM_ACQUIRE_BYTES],
    tpm_result: Result<(), RailIdError>,
    os_result: Result<(), RailIdError>,
    tpm_calls: usize,
    os_calls: usize,
}

impl ScriptedSource {
    fn new(tpm_bytes: [u8; RANDOM_ACQUIRE_BYTES], os_bytes: [u8; RANDOM_ACQUIRE_BYTES]) -> Self {
        Self {
            tpm_bytes,
            os_bytes,
            tpm_result: Ok(()),
            os_result: Ok(()),
            tpm_calls: 0,
            os_calls: 0,
        }
    }
}

impl RandomSource for ScriptedSource {
    fn fill_tpm(&mut self, out: &mut [u8]) -> Result<(), RailIdError> {
        self.tpm_calls += 1;
        self.tpm_result?;
        out.copy_from_slice(&self.tpm_bytes[..out.len()]);
        Ok(())
    }

    fn fill_os(&mut self, out: &mut [u8]) -> Result<(), RailIdError> {
        self.os_calls += 1;
        self.os_result?;
        out.copy_from_slice(&self.os_bytes[..out.len()]);
        Ok(())
    }
}

fn sequential() -> [u8; RANDOM_ACQUIRE_BYTES] {
    let mut bytes = [0u8; RANDOM_ACQUIRE_BYTES];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = index as u8;
    }
    bytes
}

#[test]
fn known_encoding_vectors() {
    let cases = [
        (0u64, [0u8; RANDOM_ACQUIRE_BYTES], ZERO_ID),
        (
            1,
            [0u8; RANDOM_ACQUIRE_BYTES],
            "AAAAAABAAAAAAAAAAAAAAAAAAAAAAAAA",
        ),
        (
            TIMESTAMP_MAX,
            [0u8; RANDOM_ACQUIRE_BYTES],
            "_______AAAAAAAAAAAAAAAAAAAAAAAAA",
        ),
        (
            TIMESTAMP_MAX,
            [0xffu8; RANDOM_ACQUIRE_BYTES],
            "________________________________",
        ),
        (
            123_456_789,
            [0u8; RANDOM_ACQUIRE_BYTES],
            "AAHW80VAAAAAAAAAAAAAAAAAAAAAAAAA",
        ),
    ];

    for (timestamp, random, expected) in cases {
        let id = RailId::from_parts(timestamp, random).unwrap();
        assert_eq!(id.encode(), expected, "timestamp {timestamp}");
        assert_eq!(RailId::decode(expected).unwrap(), id);
    }
}

#[test]
fn layout_round_trips_many_values() {
    let mut state = 0x9e37_79b9_7f4a_7c15u64;

    for _ in 0..10_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;

        let timestamp = state & TIMESTAMP_MAX;
        let mut random = [0u8; RANDOM_ACQUIRE_BYTES];
        for byte in &mut random {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *byte = state as u8;
        }

        let id = from_parts(timestamp, random).unwrap();
        let mut expected_random = random;
        expected_random[0] &= 0x3f;

        assert_eq!(id.timestamp(), timestamp);
        assert_eq!(id.random(), expected_random);
        assert_eq!(decode(&id.encode()).unwrap(), id);
    }
}

#[test]
fn encoded_and_binary_lengths_are_fixed() {
    let id = RailId::from_parts(42, sequential()).unwrap();
    assert_eq!(id.encode().len(), 32);
    assert_eq!(id.as_bytes().len(), 24);
    assert_eq!(id.to_bytes().len(), 24);
    assert_eq!(id.to_string(), id.encode());
    assert!(format!("{id:?}").contains(&id.encode()));
    assert_eq!(id.encode().parse::<RailId>().unwrap(), id);
}

#[test]
fn round_trip_preserves_timestamp_and_random_fields() {
    let mut random = [0xffu8; RANDOM_ACQUIRE_BYTES];
    random[1] = 0x01;
    random[18] = 0x80;

    let id = RailId::from_parts(123_456_789, random).unwrap();
    let decoded = decode(&id.encode()).unwrap();

    assert_eq!(decoded, id);
    assert_eq!(decoded.timestamp(), 123_456_789);
    assert_eq!(decoded.timestamp_millis(), 12_345_678_900);

    // The two most significant bits of the 19-byte random view are not part of
    // the 150-bit field.
    let random_view = decoded.random();
    assert_eq!(random_view[0], 0x3f);
    assert_eq!(random_view[1], 0x01);
    assert_eq!(random_view[18], 0x80);
}

#[test]
fn timestamp_is_the_upper_42_bits() {
    let id = RailId::from_parts(TIMESTAMP_MAX, [0u8; RANDOM_ACQUIRE_BYTES]).unwrap();
    assert_eq!(id.timestamp(), TIMESTAMP_MAX);
    assert_eq!(id.timestamp_millis(), TIMESTAMP_MAX * 100);

    // Timestamp bits 1..=0 live in the top two bits of byte 5.
    assert_eq!(id.as_bytes()[5] >> 6, 0b11);
}

#[test]
fn random_top_and_bottom_bits_land_in_the_right_places() {
    let mut random = [0u8; RANDOM_ACQUIRE_BYTES];
    random[0] = 0x20; // bit 149 of the random field
    let id = RailId::from_parts(0, random).unwrap();
    assert_eq!(id.as_bytes()[5], 0x20);
    assert_eq!(id.timestamp(), 0);

    let mut random = [0u8; RANDOM_ACQUIRE_BYTES];
    random[18] = 0x01; // least significant random bit
    let id = RailId::from_parts(0, random).unwrap();
    assert_eq!(id.as_bytes()[23], 0x01);
    assert_eq!(id.timestamp(), 0);
}

#[test]
fn decode_rejects_wrong_lengths() {
    assert_eq!(RailId::decode(""), Err(RailIdError::InvalidLength));
    assert_eq!(
        RailId::decode("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
        Err(RailIdError::InvalidLength)
    );
    assert_eq!(
        RailId::decode("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
        Err(RailIdError::InvalidLength)
    );
}

#[test]
fn decode_rejects_invalid_alphabet_and_padding() {
    // '+' and '/' are standard Base64, not Base64URL.
    assert_eq!(
        RailId::decode("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA+A"),
        Err(RailIdError::InvalidEncoding)
    );
    assert_eq!(
        RailId::decode("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/A"),
        Err(RailIdError::InvalidEncoding)
    );
    // Padding is not permitted.
    assert_eq!(
        RailId::decode("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=="),
        Err(RailIdError::InvalidEncoding)
    );
    // Whitespace and other characters are rejected.
    assert_eq!(
        RailId::decode("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA A"),
        Err(RailIdError::InvalidEncoding)
    );
}

#[test]
fn timestamp_and_random_are_independent() {
    let base = RailId::from_parts(100, [0u8; RANDOM_ACQUIRE_BYTES]).unwrap();
    let later = RailId::from_parts(101, [0u8; RANDOM_ACQUIRE_BYTES]).unwrap();
    let random = RailId::from_parts(100, sequential()).unwrap();

    assert_eq!(base.timestamp(), 100);
    assert_eq!(later.timestamp(), 101);
    assert_eq!(random.timestamp(), 100);
    assert_ne!(base.random(), random.random());
}

#[test]
fn ordering_is_binary_and_timestamp_first() {
    let first = RailId::from_parts(1, [0u8; RANDOM_ACQUIRE_BYTES]).unwrap();
    let second = RailId::from_parts(2, [0u8; RANDOM_ACQUIRE_BYTES]).unwrap();
    let later_random = RailId::from_parts(1, sequential()).unwrap();

    assert!(first < second);
    assert!(first < later_random);
    assert_eq!(
        first.as_bytes().cmp(second.as_bytes()),
        std::cmp::Ordering::Less
    );
}

#[test]
fn hybrid_profile_xors_tpm_and_os_before_reduction() {
    let tpm = sequential();
    let os = [0xaau8; RANDOM_ACQUIRE_BYTES];
    let mut source = ScriptedSource::new(tpm, os);

    let id = generate_with_source(Profile::TpmHybrid, 123_456_789, &mut source).unwrap();

    // Reference values computed independently from the v1 bit layout.
    assert_eq!(hex(&id.random()), "2aaaea2a6babeb2b68a8e82869a9e9296eaeee");
    assert_eq!(id.encode(), "AAHW80Vqquoqa6vrK2io6Chpqekpbq7u");
    assert_eq!(id.timestamp(), 123_456_789);
    assert_eq!(source.tpm_calls, 1);
    assert_eq!(source.os_calls, 1);
}

#[test]
fn tpm_failure_does_not_fall_back_to_os_randomness() {
    let mut source = ScriptedSource::new(sequential(), [0x5au8; RANDOM_ACQUIRE_BYTES]);
    source.tpm_result = Err(RailIdError::TpmRandomFailure);

    assert_eq!(
        generate_with_source(Profile::TpmHybrid, 0, &mut source),
        Err(RailIdError::TpmRandomFailure)
    );
    assert_eq!(source.tpm_calls, 1);
    assert_eq!(source.os_calls, 0, "OS source must not be consulted");

    let mut source = ScriptedSource::new(sequential(), [0x5au8; RANDOM_ACQUIRE_BYTES]);
    source.tpm_result = Err(RailIdError::TpmUnavailable);
    assert_eq!(
        generate_with_source(Profile::TpmHybrid, 0, &mut source),
        Err(RailIdError::TpmUnavailable)
    );
    assert_eq!(source.os_calls, 0);
}

#[test]
fn os_failure_is_reported() {
    let mut source = ScriptedSource::new(sequential(), [0u8; RANDOM_ACQUIRE_BYTES]);
    source.os_result = Err(RailIdError::OsRandomFailure);

    assert_eq!(
        generate_with_source(Profile::TpmHybrid, 0, &mut source),
        Err(RailIdError::OsRandomFailure)
    );
    assert_eq!(source.os_calls, 1);
}

#[test]
fn degraded_profile_uses_os_only_and_never_the_tpm() {
    let os = sequential();
    let mut source = ScriptedSource::new([0xffu8; RANDOM_ACQUIRE_BYTES], os);

    let id = generate_with_source(Profile::OsOnlyDegraded, 1, &mut source).unwrap();

    assert_eq!(source.tpm_calls, 0);
    assert_eq!(source.os_calls, 1);
    assert_eq!(hex(&id.random()), "00004080c1014181c2024282c3034383c40444");
    assert_eq!(id.encode(), "AAAAAABAAECAwQFBgcICQoLDA0ODxARE");
    assert!(Profile::OsOnlyDegraded.is_degraded());
    assert!(!Profile::TpmHybrid.is_degraded());
}

#[test]
fn timestamp_range_is_enforced_before_randomness_is_read() {
    assert!(from_parts(TIMESTAMP_MAX, [0u8; RANDOM_ACQUIRE_BYTES]).is_ok());
    assert_eq!(
        from_parts(TIMESTAMP_MAX + 1, [0u8; RANDOM_ACQUIRE_BYTES]),
        Err(RailIdError::TimestampOutOfRange)
    );

    let mut source = ScriptedSource::new(sequential(), sequential());
    assert_eq!(
        generate_with_source(Profile::TpmHybrid, TIMESTAMP_MAX + 1, &mut source),
        Err(RailIdError::TimestampOutOfRange)
    );
    assert_eq!(source.tpm_calls, 0);
    assert_eq!(source.os_calls, 0);
}

#[test]
fn many_generated_ids_are_unique_and_well_formed() {
    let mut ids = HashSet::new();
    let mut source = railid::SystemRandomSource;

    for _ in 0..2_000 {
        let id = generate_with_source(Profile::OsOnlyDegraded, 7, &mut source).unwrap();
        assert_eq!(id.encode().len(), 32);
        assert_eq!(id.timestamp(), 7);
        assert_eq!(id.random()[0] & 0xc0, 0);
        assert!(ids.insert(id.to_bytes()), "duplicate RailID generated");
    }
}

#[cfg(any(
    all(feature = "tpm", target_os = "windows"),
    all(feature = "tpm", target_os = "linux")
))]
#[test]
fn tpm_generation_is_thread_safe_when_required() {
    if std::env::var_os("RAILID_REQUIRE_TPM").is_none() {
        return;
    }

    let mut handles = Vec::new();
    for _ in 0..8 {
        handles.push(std::thread::spawn(|| {
            for _ in 0..10 {
                railid::generate().expect("concurrent TPM generation failed");
            }
        }));
    }
    for handle in handles {
        handle.join().expect("TPM worker panicked");
    }
}

#[cfg(target_os = "windows")]
#[test]
fn windows_tpm_profile_generates_or_fails_closed() {
    match railid::generate() {
        Ok(id) => {
            let encoded = id.encode();
            assert_eq!(encoded.len(), 32);
            assert!(encoded
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'));
            assert_eq!(RailId::decode(&encoded).unwrap(), id);
        }
        Err(error) => assert!(matches!(
            error,
            RailIdError::TpmUnavailable | RailIdError::TpmRandomFailure
        )),
    }

    // Opt-in strict check for machines expected to expose a TPM:
    //   set RAILID_REQUIRE_TPM=1 && cargo test windows_tpm_profile
    if std::env::var_os("RAILID_REQUIRE_TPM").is_some() {
        assert!(
            railid::generate().is_ok(),
            "RAILID_REQUIRE_TPM is set but the Windows TBS/TPM backend failed"
        );
    }

    assert_eq!(
        railid::generate_os_only().map(|id| id.encode().len()),
        Ok(32)
    );
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

/// Entropy source that records requested sizes and emits sequential bytes.
struct BulkSource {
    tpm_calls: Vec<usize>,
    os_calls: Vec<usize>,
    tpm_byte: u8,
    os_byte: u8,
    fail_tpm: bool,
    fail_os: bool,
}

impl BulkSource {
    fn new() -> Self {
        Self {
            tpm_calls: Vec::new(),
            os_calls: Vec::new(),
            tpm_byte: 0,
            os_byte: 0x80,
            fail_tpm: false,
            fail_os: false,
        }
    }
}

impl RandomSource for BulkSource {
    fn fill_tpm(&mut self, out: &mut [u8]) -> Result<(), RailIdError> {
        self.tpm_calls.push(out.len());
        if self.fail_tpm {
            return Err(RailIdError::TpmRandomFailure);
        }
        for byte in out {
            *byte = self.tpm_byte;
            self.tpm_byte = self.tpm_byte.wrapping_add(1);
        }
        Ok(())
    }

    fn fill_os(&mut self, out: &mut [u8]) -> Result<(), RailIdError> {
        self.os_calls.push(out.len());
        if self.fail_os {
            return Err(RailIdError::OsRandomFailure);
        }
        for byte in out {
            *byte = self.os_byte;
            self.os_byte = self.os_byte.wrapping_add(1);
        }
        Ok(())
    }
}

#[test]
fn bulk_os_generation_is_complete_unique_and_round_trips() {
    let count = 20_000;
    let ids = generate_many(count, Profile::OsOnlyDegraded).unwrap();
    assert_eq!(ids.len(), count);
    assert!(bulk_worker_count(count) >= 1);

    let mut seen = HashSet::new();
    for id in &ids {
        assert_eq!(id.encode().len(), 32);
        assert_eq!(RailId::decode(&id.encode()).unwrap(), *id);
        assert!(seen.insert(id.to_bytes()), "duplicate RailID in bulk batch");
    }

    let strings = generate_many_strings(5_000, Profile::OsOnlyDegraded).unwrap();
    assert_eq!(strings.len(), 5_000);
    for encoded in &strings {
        assert_eq!(encoded.len(), 32);
        assert!(RailId::decode(encoded).is_ok());
    }

    assert!(generate_many(0, Profile::OsOnlyDegraded)
        .unwrap()
        .is_empty());
    assert_eq!(bulk_worker_count(0), 0);
    assert_eq!(bulk_worker_count(1), 1);
}

#[test]
fn bulk_tpm_requests_64_bytes_and_uses_19_per_id() {
    let mut source = BulkSource::new();
    let ids = generate_many_with_source(10, Profile::TpmHybrid, &mut source).unwrap();

    assert_eq!(ids.len(), 10);
    assert_eq!(source.tpm_calls.len(), 3);
    assert!(source
        .tpm_calls
        .iter()
        .all(|size| *size == TPM_BULK_READ_BYTES));
    assert_eq!(source.os_calls, vec![RANDOM_ACQUIRE_BYTES; 10]);

    for id in &ids {
        assert_eq!(RailId::decode(&id.encode()).unwrap(), *id);
    }
}

#[test]
fn bulk_tpm_single_id_uses_a_single_19_byte_read() {
    let mut source = BulkSource::new();
    let ids = generate_many_with_source(1, Profile::TpmHybrid, &mut source).unwrap();

    assert_eq!(ids.len(), 1);
    assert_eq!(source.tpm_calls, vec![RANDOM_ACQUIRE_BYTES]);
    assert_eq!(source.os_calls, vec![RANDOM_ACQUIRE_BYTES]);
}

#[test]
fn bulk_tpm_failure_does_not_fall_back() {
    let mut source = BulkSource::new();
    source.fail_tpm = true;
    assert_eq!(
        generate_many_with_source(5, Profile::TpmHybrid, &mut source),
        Err(RailIdError::TpmRandomFailure)
    );
    assert!(source.os_calls.is_empty());

    let mut source = BulkSource::new();
    source.fail_os = true;
    assert_eq!(
        generate_many_with_source(5, Profile::TpmHybrid, &mut source),
        Err(RailIdError::OsRandomFailure)
    );
    assert_eq!(source.tpm_calls, vec![TPM_BULK_READ_BYTES]);
}

#[test]
fn bulk_zero_count_does_not_touch_entropy_sources() {
    let mut source = BulkSource::new();
    let ids = generate_many_with_source(0, Profile::TpmHybrid, &mut source).unwrap();
    assert!(ids.is_empty());
    assert!(source.tpm_calls.is_empty());
    assert!(source.os_calls.is_empty());
}

#[test]
fn streaming_strings_are_delivered_and_valid() {
    use std::sync::Mutex;

    let seen = Mutex::new(HashSet::new());
    for_each_many_strings(5_000, Profile::OsOnlyDegraded, |encoded| {
        assert_eq!(encoded.len(), 32);
        assert!(RailId::decode(&encoded).is_ok());
        seen.lock().unwrap().insert(encoded);
    })
    .unwrap();

    assert_eq!(seen.lock().unwrap().len(), 5_000);
}

#[test]
fn streaming_encoded_strings_are_delivered() {
    use std::sync::Mutex;

    let seen = Mutex::new(HashSet::new());
    for_each_many_encoded(5_000, Profile::OsOnlyDegraded, |encoded| {
        assert_eq!(encoded.len(), 32);
        assert!(RailId::decode(encoded).is_ok());
        seen.lock().unwrap().insert(encoded.to_owned());
    })
    .unwrap();

    assert_eq!(seen.lock().unwrap().len(), 5_000);
}

#[test]
fn encoded_array_matches_owned_encoding() {
    let id = from_parts(123_456, sequential()).unwrap();
    let array = id.encode_array();
    assert_eq!(array.as_str(), id.encode());
    assert_eq!(encode_array(&id).as_str(), id.encode());
    assert_eq!(array.as_bytes().len(), 32);
}

#[test]
fn encoded_batch_is_contiguous_and_valid() {
    let count = 5_000;
    let batch = generate_many_encoded_batch(count, Profile::OsOnlyDegraded).unwrap();
    assert_eq!(batch.len(), count);
    assert_eq!(batch.as_bytes().len(), count * 32);

    let mut seen = HashSet::new();
    for (index, encoded) in batch.iter().enumerate() {
        assert_eq!(batch.get(index), Some(encoded));
        assert_eq!(encoded.len(), 32);
        let id = RailId::decode(encoded).unwrap();
        assert!(seen.insert(id.to_bytes()), "duplicate ID in encoded batch");
    }
    assert_eq!(seen.len(), count);
}

#[test]
fn encoded_batch_rejects_capacity_overflow() {
    assert_eq!(
        generate_many_encoded_batch(usize::MAX, Profile::OsOnlyDegraded),
        Err(RailIdError::CapacityOverflow)
    );
}
