// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import nacl from "tweetnacl";
import { Deserializer } from "../bcs/deserializer";
import { Serializer } from "../bcs/serializer";
import { Hex } from "../core/hex";
import { HexInput } from "../types";
import { PublicKey, PrivateKey, Signature } from "./asymmetric_crypto";

export class Ed25519PublicKey extends PublicKey {
  // Correct length of the public key in bytes (Uint8Array)
  static readonly LENGTH: number = 32;

  // Hex value of the public key
  private readonly key: Hex;

  /**
   * Create a new PublicKey instance from a Uint8Array or String.
   *
   * @param args.bytes A HexInput (string or Uint8Array)
   */
  constructor(args: { hexInput: HexInput }) {
    super();

    const hex = Hex.fromHexInput(args);
    if (hex.toUint8Array().length !== Ed25519PublicKey.LENGTH) {
      throw new Error(`PublicKey length should be ${Ed25519PublicKey.LENGTH}`);
    }
    this.key = hex;
  }

  /**
   * Get the public key in bytes (Uint8Array).
   *
   * @returns Uint8Array representation of the public key
   */
  toUint8Array(): Uint8Array {
    return this.key.toUint8Array();
  }

  /**
   * Get the public key as a hex string with the 0x prefix.
   *
   * @returns string representation of the public key
   */
  toString(): string {
    return this.key.toString();
  }

  /**
   * Verifies a signed data with a public key
   * @param args.data a signed message
   * @param args.signature the signature of the message
   */
  verifySignature(args: { message: HexInput; signature: Ed25519Signature }): boolean {
    const { message, signature } = args;
    const rawMessage = Hex.fromHexInput({ hexInput: message }).toUint8Array();
    const rawSignature = Hex.fromHexInput({ hexInput: signature.toUint8Array() }).toUint8Array();
    return nacl.sign.detached.verify(rawMessage, rawSignature, this.key.toUint8Array());
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.key.toUint8Array());
  }

  static deserialize(deserializer: Deserializer): PublicKey {
    const bytes = deserializer.deserializeBytes();
    return new Ed25519PublicKey({ hexInput: bytes });
  }

  // eslint-disable-next-line class-methods-use-this,@typescript-eslint/no-unused-vars
  deserialize(deserializer: Deserializer): PublicKey {
    throw new Error("Not implemented");
  }
}

export class Ed25519PrivateKey extends PrivateKey {
  // Correct length of the private key in bytes (Uint8Array)
  static readonly LENGTH: number = 32;

  // Private and public key pair
  private readonly signingKeyPair: nacl.SignKeyPair;

  /**
   * Create a new PrivateKey instance from a Uint8Array or String.
   *
   * @param value HexInput (string or Uint8Array)
   */
  constructor(args: { hexInput: HexInput }) {
    super();

    const privateKeyHex = Hex.fromHexInput(args);
    if (privateKeyHex.toUint8Array().length !== Ed25519PrivateKey.LENGTH) {
      throw new Error(`PrivateKey length should be ${Ed25519PrivateKey.LENGTH}`);
    }

    // Create keyPair from Private key in Uint8Array format
    const keyPair = nacl.sign.keyPair.fromSeed(privateKeyHex.toUint8Array().slice(0, Ed25519PrivateKey.LENGTH));
    this.signingKeyPair = keyPair;
  }

  /**
   * Get the private key in bytes (Uint8Array).
   *
   * @returns Uint8Array representation of the private key
   */
  toUint8Array(): Uint8Array {
    return this.signingKeyPair.secretKey.slice(0, Ed25519PrivateKey.LENGTH);
  }

  /**
   * Get the private key as a hex string with the 0x prefix.
   *
   * @returns string representation of the private key
   */
  toString(): string {
    return Hex.fromHexInput({ hexInput: this.toUint8Array() }).toString();
  }

  /**
   * Sign the given message with the private key.
   *
   * @param args.message in HexInput format
   * @returns Signature
   */
  sign(args: { message: HexInput }): Ed25519Signature {
    const hex = Hex.fromHexInput({ hexInput: args.message });
    const signature = nacl.sign.detached(hex.toUint8Array(), this.signingKeyPair.secretKey);
    return new Ed25519Signature({ hexInput: signature });
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.toUint8Array());
  }

  // TODO: Update this in interface to be static, then remove this method
  deserialize(deserializer: Deserializer): Ed25519PrivateKey {
    throw new Error("Method not implemented.");
  }

  static deserialize(deserializer: Deserializer): Ed25519PrivateKey {
    const bytes = deserializer.deserializeBytes();
    return new Ed25519PrivateKey({ hexInput: bytes });
  }

  /**
   * Generate a new random private key.
   *
   * @returns Ed25519PrivateKey
   */
  static generate(): Ed25519PrivateKey {
    const keyPair = nacl.sign.keyPair();
    return new Ed25519PrivateKey({ hexInput: keyPair.secretKey.slice(0, Ed25519PrivateKey.LENGTH) });
  }

  publicKey(): PublicKey {
    const bytes = this.signingKeyPair.publicKey;
    return new Ed25519PublicKey({ hexInput: bytes });
  }
}

/**
 * The product of signing a message with a private key.
 */
export class Ed25519Signature extends Signature {
  // Correct length of the signature in bytes (Uint8Array)
  static readonly LENGTH = 64;

  // Hex value of the signature
  private readonly data: Hex;

  constructor(args: { hexInput: HexInput }) {
    super();
    const hex = Hex.fromHexInput(args);
    if (hex.toUint8Array().length !== Ed25519Signature.LENGTH) {
      throw new Error(`Signature length should be ${Ed25519Signature.LENGTH}`);
    }

    this.data = hex;
  }

  /**
   * Get the signature in bytes (Uint8Array).
   *
   * @returns Uint8Array representation of the signature
   */
  toUint8Array(): Uint8Array {
    return this.data.toUint8Array();
  }

  /**
   * Get the signature as a hex string with the 0x prefix.
   *
   * @returns string representation of the signature
   */
  toString(): string {
    return this.data.toString();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.data.toUint8Array());
  }

  // TODO: Update this in interface to be static, then remove this method
  deserialize(deserializer: Deserializer): Ed25519Signature {
    throw new Error("Method not implemented.");
  }

  static deserialize(deserializer: Deserializer): Ed25519Signature {
    const bytes = deserializer.deserializeBytes();
    return new Ed25519Signature({ hexInput: bytes });
  }
}
