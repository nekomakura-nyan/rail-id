import type { DecodedRailId } from "./index";

export type { DecodedRailId };
export type Backend = "os-only";

/** Generate one RailID with the explicitly degraded OS-only profile. */
export declare function generate(): string;

/** Same as `generate()`; kept for symmetry with the default entry point. */
export declare function generateOsOnly(): string;

/** Generate one OS-only RailID as 32 ASCII bytes, without a JS string. */
export declare function generateBuffer(): Buffer;

/** Same as `generateBuffer()`; kept for symmetry. */
export declare function generateOsOnlyBuffer(): Buffer;

/** Generate `count` IDs synchronously as strings. */
export declare function generateMany(count: number): string[];

/** Generate `count` IDs synchronously as one contiguous `count * 32` Buffer. */
export declare function generateManyEncoded(count: number): Buffer;

/** Generate `count` IDs asynchronously as strings. */
export declare function generateManyAsync(count: number): Promise<string[]>;

/** Generate `count` IDs asynchronously as one contiguous `count * 32` Buffer. */
export declare function generateManyEncodedAsync(count: number): Promise<Buffer>;

/** Decode and validate a RailID. */
export declare function decode(input: string): DecodedRailId;

/** RailID crate version. */
export declare function version(): string;

/** Always `"os-only"` for this entry point. */
export declare function backend(): Backend;
