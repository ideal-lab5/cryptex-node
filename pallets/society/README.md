# Society/DKG Pallet 

This is a work in progress. This pallet will allow addresses to form into societies that share a distributed public/private keypair (over BLS12-381). With this, the society is able to participate in activities that require threshold signatures or threshold encryption.

## Extrinsics

- create_society
- join_society (keygen)
- Encryption
  - submit_decryption_share 
- Threshold Signatures
  - submit_signed_message (for threshold signatures)

## Offchain
- encrypt_message (callable by anybody, calculates the society's pubkey on the fly given input)
  - probably done in browser/client using dkg lib bundled as wasm

## Usage
