# Society/DKG Pallet 

This is a work in progress. This pallet will allow addresses to form into societies that share a distributed public/private keypair (over BLS12-381). With this, the society is able to participate in activities that require threshold signatures or threshold encryption.

It should be noted that the input to this pallet is expected to be generated using the dkg_wasm library called from a browser, though technically this isn't required.

It should also be noted that this pallet does **NOT** enable a decentralized KEM (as Blind DKG would). Additionally, it is not privacy preserving.


Society formation happens in several phase:

```
                                                                                      
                +   <--d_0 -->    +     <--d_1-->      +            <--d_2-->
+-------------------------------------------------------------------------------------+
Phase::Create   +   Phase::Join   +   Phase::Submit    +    Phase::(Verify + ) Dispute    
```

First, a potential society is created. For the next `d_0` blocks, invited members have the opportunity to commit to joining by issuing a BLS12-381 public key onchain. Afer the deadline passes, for the next d_1 blocks, participants submit encrypted shares and commitments on chain. And finally, in the last d_2 blocks, nodes verify (offchain) and dispute any invalid shares they encountered.

## Extrinsics

### Permissionless
- `create_society`(id, name, threshold, members, deadline): create a new onchain soceity, mapping an id to a 'society' struct, including founder, members, and threshold. Potential members have until the specified deadline  to join. Once the deadline is reached, if a threshold has not participated then the society should 'fail'. If at least a threshold has 'joined', then it starts the next phase where they submit shares and commitments. 
- `join_society`(id, pubkey): join a society if you haven't done so by submitting a new public key (can be any BLS12-381 keypair)
- `submit_shares`(pubkey -> (encrypted_share, commitment)): only callable during the submission period, submit encrypted shares and commitments on chain for each participant who joined.
- `dispute_share`

### Permissioned
- ?

- Encryption
  - submit_decryption_share 
- Threshold Signatures
  - submit_signed_message (for threshold signatures)

## Offchain
- encrypt_message (callable by anybody, calculates the society's pubkey on the fly given input)
  - probably done in browser/client using dkg lib bundled as wasm

## Usage

``` rust
impl pallet_society::Config for Runtime {
  type RuntimeEvent = RuntimeEvent;
}
```