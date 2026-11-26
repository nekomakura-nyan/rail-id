export type Backend = "tpm" | "os-only";

export interface DecodedRailId {
  /** Unix timestamp in milliseconds. */
  timestampMs: number;
  /** 150-bit random field as 38 lowercase hex characters. */
  randomHex: string;
  /** Canonical 32-character Base64URL encoding. */
  encoded: string;
}

/** Generate one RailID with the standard TPM hybrid profile. */
export declare function generate(): string;

/** Generate one RailID with the explicitly degraded OS-only profile. */
export declare function generateOsOnly(): string;

/** Generate `count` IDs synchronously as strings. */
export declare function generateMany(count: number, osOnly?: boolean): string[];

/** Generate `count` IDs synchronously as one contiguous `count * 32` Buffer. */
export declare function generateManyEncoded(count: number, osOnly?: boolean): Buffer;

/** Generate `count` IDs asynchronously as strings. */
export declare function generateManyAsync(count: number, osOnly?: boolean): Promise<string[]>;

/** Generate `count` IDs asynchronously as one contiguous `count * 32` Buffer. */
export declare function generateManyEncodedAsync(
  count: number,
  osOnly?: boolean,
): Promise<Buffer>;

/** Decode and validate a RailID. */
export declare function decode(input: string): DecodedRailId;

/** RailID crate version. */
export declare function version(): string;

/** Native backend compiled into the loaded addon. */
export declare function backend(): Backend;
