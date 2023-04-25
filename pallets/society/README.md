# Society/DKG Pallet 

This is a work in progress. This pallet will allow addresses to form into societies that share a distributed public/private keypair (over BLS12-381). With this, the society is able to participate in activities that require threshold signatures or threshold encryption.

It should be noted that the input to this pallet is expected to be generated using the dkg_wasm library called from a browser, though technically this isn't required.

It should also be noted that this pallet does **NOT** enable a decentralized KEM (as Blind DKG would). Additionally, it is not privacy preserving.


Society formation happens in several phase:

```
                                                                                      
                +   <--d_0 -->    +     <--d_1-->      +            <--d_2-->
+-------------------------------------------------------------------------------------+
Phase::Create   +   Phase::Commit   +   Phase::Join    +    Phase::Verify/Dispute    
```

First, a potential society is created. For the next `d_0` blocks, invited members have the opportunity to commit to joining by issuing a BLS12-381 public key onchain. Afer the deadline passes, for the next d_1 blocks, participants submit encrypted shares and commitments on chain. And finally, in the last d_2 blocks, nodes verify (offchain) and dispute any invalid shares they encountered.

## on_initialize

Each society is given two distinct deadlines. Firstly, when created, the founder specified a deadline after which participants can no longer issue commitments. After this, we arbitrarily give 10 more blocks to verify shares and submit public keys. This functionality is accomplihed using the on_initialize hook.

## Extrinsics

- `create(id, name, threshold, members, deadline)`: create a new onchain society, mapping an id to a 'society' struct, including founder, members, and threshold. Potential members have until the specified deadline  to join. Once the deadline is reached, if a threshold has not participated then the society should 'fail'. If at least a threshold has 'joined', then it starts the next phase where they submit shares and commitments. 
- `commit(id, shares_and_commitments)`: submit shares and commitments for the specific society
- `join(id, pubkey)`: submit a public key to the society, will be used in group pubkey derivation. If you submit a key, this assumes you are committing to the society. If you mark a share as invalid, then simply do not submit a public key.

## Usage

``` rust
impl pallet_society::Config for Runtime {
  type RuntimeEvent = RuntimeEvent;
}
```

## Status

- [ ] add deadlines to requests
- [ ] bound storage
- [ ] sign and verify
- [ ] encrypt/decrypt shares
