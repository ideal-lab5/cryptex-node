#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

use codec::{Encode, Decode};
use sp_std::vec::Vec;
use scale_info::TypeInfo;
use frame_support::{BoundedVec, pallet_prelude::*};
use dkg::dkg::dkg::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

type SocietyId = Vec<u8>;

#[derive(Encode, Decode, PartialEq, TypeInfo, Clone)]
pub struct Society<AccountId> {
	pub founder: AccountId,
	pub members: Vec<AccountId>,
	pub threshold: u8,
	pub name: Vec<u8>,
}

#[derive(Encode, Decode, PartialEq, TypeInfo, Clone)]
pub struct SerializedPublicKey {
	pub g1: Vec<u8>,
	pub g2: Vec<u8>,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;

	use rand_chacha::{
		ChaCha20Rng, rand_core::SeedableRng,
	};
	// TODO: do these need to be used here?
	// need to update the interface in dkg lib 
	use ark_bls12_381::{
		Bls12_381, Fr,
		G1Projective as G1, G2Affine, 
		G2Projective as G2
	};
	use ark_serialize::CanonicalSerialize;
	use ark_ec::Group;
	use ark_std::{
        ops::Mul,
        rand::Rng,
    };

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::storage]
	pub type Membership<T: Config> = StorageMap<
		_, 
		Blake2_128,
		SocietyId,
		Society<T::AccountId>,
		OptionQuery,
	>;

	#[pallet::storage]
	pub type SocietyDetails<T: Config> = StorageMap<
		_, 
		Blake2_128,
		SocietyId,
		// map the member's acct id to their public key
		Vec<(T::AccountId, SerializedPublicKey)>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {

	}

	#[pallet::error]
	pub enum Error<T> {
		SocietyAlreadyExists,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {

		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn create_society(
			origin: OriginFor<T>,
			society_id: SocietyId,
			threshold: u8,
			name: Vec<u8>,
			members: Vec<T::AccountId>,
		) -> DispatchResult {
			let founder = ensure_signed(origin)?;
			ensure!(
				Membership::<T>::get(society_id.clone()).is_none(),
				Error::<T>::SocietyAlreadyExists
			);
			Membership::<T>::insert(society_id.clone(),
				Society {
					founder: founder,
					members: members,
					threshold: threshold,
					name: name,
				});
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn join_society(
			origin: OriginFor<T>, 
			society_id: SocietyId,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;
			// for now we'll go with a simplified model
			// just generate a new polynomial, secret key, and pubkey
			// TODO: use T::Randomness::get()
			let rng = ChaCha20Rng::seed_from_u64(23u64);
			let poly = keygen(2, rng);
			let sk = calculate_secret(poly);
			let g1 = G1::generator().mul(Fr::from(11u64));
			let g2 = G2::generator().mul(Fr::from(17u64));
			let pk = calculate_pubkey(g1, g2, sk);

			// experiment 1: put the pubkey onchain as serialized + compressed
			let mut bpk1 = Vec::with_capacity(1000);
        	pk.g1.serialize_compressed(&mut bpk1).unwrap();

			let mut bpk2 = Vec::with_capacity(1000);
			pk.g2.serialize_compressed(&mut bpk2).unwrap();

			let s_pk = SerializedPublicKey {
				g1: bpk1,
				g2: bpk2,
			};

			SocietyDetails::<T>::mutate(society_id, |details| {
				details.push((who.clone(), s_pk));
			});

			Ok(())
		}

		// // request each member of a society to submit a share
		// #[pallet::call_index(2)]
		// #[pallet::weight(0)]
		// pub fn request_meeting(
		// 	origin: OriginFor<T>,
		// 	society_id: SocietyId,
		// ) -> DispatchResult {
		// 	Ok(())
		// }

		// // each member of a society can submit a share
		// #[pallet::call_index(3)]
		// #[pallet::weight(0)]
		// pub fn submit_share(
		// 	// origin: OriginFor<T>,
		// 	society_id: SocietyId,
		// ) -> DispatchResult {
		// 	Ok(())
		// }
	}
}

impl<T:Config> Pallet<T> {
	// fn calculate_society_pubkey() -> Vec<u8> {

	// }
}