# NanoID cost breakdown

| Operation | Median |
|---|---:|
| crypto.randomUUID length only | 37.83 ns |
| crypto.randomUUID copy | 249.99 ns |
| uuid.v4 length only | 56.09 ns |
| uuid.v4 copy | 302.85 ns |
| nanoid length only | 34.80 ns |
| nanoid copy | 93.40 ns |
| RailID OS-only length only | 179.41 ns |
| RailID OS-only copy | 213.25 ns |
| getRandomValues(21) only | 1132.27 ns |
| prebuilt pool substring copy | 53.89 ns |
| 32 KiB pool refill (per refill) | 39500.00 ns (25.32 ns/ID at 1,560 IDs/refill) |

- Iterations per sample: 500,000, samples: 5, warmups: 2.
- `length only` may not force V8 to flatten the string; `copy` writes into a fixed ASCII buffer.
- Sink: -676372304
