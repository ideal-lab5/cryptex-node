#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

use codec::{Encode, Decode};
use sp_std::vec::Vec;
use scale_info::TypeInfo;
use frame_support::{BoundedVec, pallet_prelude::*};
use dkg_core::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

type SocietyId = Vec<u8>;

#[derive(Encode, Decode, PartialEq, TypeInfo, Clone, Debug)]
pub struct Society<AccountId> {
	pub founder: AccountId,
	pub members: Vec<AccountId>,
	pub threshold: u8,
	pub name: Vec<u8>,
}

#[derive(Encode, Decode, PartialEq, TypeInfo, Clone)] 
pub struct Capsule<AccountId> {
	pub recipient: AccountId,
	pub sender: AccountId,
	pub share: Vec<u8>,
	pub commitment: Vec<u8>
}

#[derive(Encode, Decode, PartialEq, TypeInfo, Clone, Debug)] 
pub struct Share {
	pub share: Vec<u8>,
	pub commitment: Vec<u8>
}

#[derive(Encode, Decode, PartialEq, TypeInfo, Clone)] 
pub struct TransactionRequest<AccountId, Balance> {
	pub recipient: AccountId,
	pub amount: Balance,
}

#[derive(Encode, Decode, PartialEq, TypeInfo, Clone, Debug)]
pub enum Phase {
	Commit,
	Join,
	Active,
	Failed,
}

#[derive(Encode, Decode, PartialEq, TypeInfo, Clone)]
pub enum MemberStatus {
	Invitee,
	Committed,
	Active,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;

	use rand_chacha::{
		ChaCha20Rng, rand_core::SeedableRng,
	};
	use ark_bls12_381::{
		Fr, G1Projective as G1, 
		G2Projective as G2
	};
	use ark_crypto_primitives::signature::schnorr::Signature;
	use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
	use ark_ec::Group;
	use ark_std::{
        ops::Mul,
        rand::Rng,
    };
	use dkg_core::*;
	use frame_support::weights::Weight;

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
	pub type Societies<T: Config> = StorageMap<
		_, 
		Blake2_128,
		SocietyId,
		Society<T::AccountId>,
		OptionQuery,
	>;

	#[pallet::storage]
	pub type SharesAndCommitments<T: Config> = StorageMap<
		_, 
		Blake2_128,
		SocietyId,
		Vec<Capsule<T::AccountId>>,
		ValueQuery,
	>;

	/// commitments to participate in the society
	#[pallet::storage]
	pub type RSVP<T: Config> = StorageMap<
		_, 
		Blake2_128,
		SocietyId,
		// map the member's acct id to their public key
		Vec<(T::AccountId, Vec<u8>)>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type Membership<T: Config> = StorageDoubleMap<
		_,
		Blake2_128,
		T::AccountId,
		Blake2_128,
		MemberStatus,
		Vec<SocietyId>,
		ValueQuery,
	>;

	/// map block number to societies who have join deadlines on that block
	#[pallet::storage]
	pub type SocietyStatus<T: Config> = StorageMap<
		_,
		Blake2_128,
		SocietyId,
		Vec<(T::BlockNumber, Phase)>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type Signatures<T: Config> = StorageMap<
		_,
		Blake2_128,
		SocietyId,
		Vec<(T::AccountId, (Vec<u8>, Vec<u8>))>, // TODO: expose serializable wrapper from dkg lib
		ValueQuery,
	>;

	#[pallet::storage]
	pub type Deadlines<T: Config> = StorageMap<
		_,
		Blake2_128,
		T::BlockNumber,
		Vec<SocietyId>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type WalletTransactionQueue<T: Config> = StorageMap<
		_,
		Blake2_128,
		T::BlockNumber,
		Vec<Tra>,
		ValueQuery,
	>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		
		fn on_initialize(n: T::BlockNumber) -> Weight {
			let societies = Deadlines::<T>::get(n);
			for society_id in societies.iter() {
				Self::try_set_join(n, society_id.clone())
					.map_err(|_| {
						SocietyStatus::<T>::mutate(society_id.clone(), |v| {
							v.push((current_block_number, Phase::Failed));
						});
					});
				// 10 blocks arbitrarily for now
				let ten = T::BlockNumber::from(10u32);
				if n > ten {
					let d: T::BlockNumber = n - ten;
					Self::try_set_commit(d, society_id.clone())
						.map_err(|_| {
							// TODO: could extend the deadline here
							SocietyStatus::<T>::mutate(society_id.clone(), |v| {
								v.push((current_block_number, Phase::Failed));
							});
						});
				}
				
			}
			
			Weight::zero()
		}
	}
 
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		CreatedSociety,
		ClosedSociety,
		StartedSociety,
		CommitedToSociety,
		JoinedSociety,
	}

	#[pallet::error]
	pub enum Error<T> {
		SocietyAlreadyExists,
		InvalidPublicKey,
		NotMember,
		NotFounder,
		ThresholdNotReached,
		NotCommitPhase,
		NotJoinPhase,
		InvalidMembershipChange,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {

		/// Create a new society.
		/// 
		/// * `society_id`: The id of the society, can be anything
		/// * `threshold`: The threshold of the society;
		/// 			   The minimum number of signatures required to count as a full signature.
		/// * `name`: The name of the society, can be anything
		/// * `deadline`: The block number after which the society's membership closes
		/// * `members`: The members of the society
		/// 
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn create(
			origin: OriginFor<T>,
			society_id: SocietyId,
			threshold: u8,
			name: Vec<u8>,
			deadline: T::BlockNumber,
			members: Vec<T::AccountId>, // note: the order of the members defined here determines each member's 'slot'
		) -> DispatchResult {
			let founder = ensure_signed(origin)?;
			ensure!(
				Societies::<T>::get(society_id.clone()).is_none(),
				Error::<T>::SocietyAlreadyExists
			);
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			for member in members.iter() {
				Membership::<T>::mutate(
					member.clone(), 
						MemberStatus::Invitee, 
							|v| v.push(society_id.clone()));
			}
			Societies::<T>::insert(society_id.clone(),
				Society {
					founder: founder,
					members: members.clone(),
					threshold: threshold,
					name: name,
				});
			// tell the members that they can commit to participating
			// (by providing shares + commitments)
			SocietyStatus::<T>::mutate(society_id.clone(), |v| {
				v.push((current_block_number, Phase::Commit));
			});
			Deadlines::<T>::mutate(deadline, |v| v.push(society_id.clone()));
			Self::deposit_event(Event::<T>::CreatedSociety);
			Ok(())
		}

		/// submit encrypted shares and commitments for a specific society
		/// in which you are a member. You should have `MemberStatus::Invitee`
		/// and the society should be in `Phase::Commit`
		/// 
		/// * `society_id`: the unique society id
		/// * `share_and_commitments`: A vec of shares and associated commitments
		/// 
		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn commit(
			origin: OriginFor<T>, 
			society_id: SocietyId,
			shares_and_commitments: Vec<Share>,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;
			let society = Self::try_get_society(society_id.clone(), who.clone())?;
			// check that the society is in the 'commit' phase
			Self::check_society_phase(society_id.clone(), Phase::Commit)?;
			// TODO: verify that you have submitted the proper number of shares/handle error
			society.clone().members.iter().zip(
				shares_and_commitments.iter()
			).for_each(|(a, b)| {
				SharesAndCommitments::<T>::mutate(society_id.clone(), |v| {
					v.push(Capsule {
						recipient: a.clone(),
						sender: who.clone(), 
						share: b.share.clone(), 
						commitment: b.commitment.clone(),
					});
				});	
			});
			// update membership map
			Self::update_membership(
				who.clone(), 
				society_id.clone(),
				MemberStatus::Invitee, 
				MemberStatus::Committed,
			)?;
			Self::deposit_event(Event::<T>::CommitedToSociety);
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn join(
			origin: OriginFor<T>, 
			society_id: SocietyId,
			compressed_g2: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;
			let society = Self::try_get_society(society_id.clone(), who.clone())?;
			// check that the society is in the 'commit' phase
			Self::check_society_phase(society_id.clone(), Phase::Commit)?;
			// just to verify it can be turned into a group element later on
			// this is probably not necessary, but leaving for now
			ark_bls12_381::G2Projective::deserialize_compressed(&compressed_g2[..])
				.map_err(|e| {
					return Error::<T>::InvalidPublicKey;
				});
			RSVP::<T>::mutate(society_id.clone(), |rsvps| {
				rsvps.push((who.clone(), compressed_g2));
			});
			// update membership map
			Self::update_membership(
				who.clone(), 
				society_id.clone(),
				MemberStatus::Committed, 
				MemberStatus::Active,
			)?;
			Self::deposit_event(Event::<T>::JoinedSociety);
			Ok(())
		}

		/// submit a signature for a society
		#[pallet::call_index(3)]
		#[pallet::weight(0)]
		pub fn submit_signature(
			origin: OriginFor<T>,
			society_id: SocietyId, 
			prover_response: Vec<u8>,
			verifier_challenge: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// TODO: verifications
			Signatures::<T>::mutate(society_id, |v| {
				v.push((who.clone(), (prover_response, verifier_challenge)));
			});
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {

	fn update_membership(
		who: T::AccountId,
		society_id: SocietyId,
		old_status: MemberStatus, 
		new_status: MemberStatus
	) -> DispatchResult {
		// remove old status
		Membership::<T>::mutate(
			who.clone(), old_status, |v| {
				// this is kind of weird
				let index = v.iter()
					.position(|x| *x == society_id.clone())
					.ok_or(Error::<T>::InvalidMembershipChange)
					.map_err(|e| return Error::<T>::InvalidMembershipChange)
					.unwrap();
				v.remove(index);
			});
		// add new status
		Membership::<T>::mutate(
			who.clone(), new_status, |v| v.push(society_id.clone()));
		Ok(())
	}

	fn try_get_society(
		society_id: SocietyId,
		who: T::AccountId,
	) -> Result<Society<T::AccountId>, Error<T>> {
		// ensure member of societys
		let maybe_society = Societies::<T>::get(society_id.clone());
		ensure!(
			maybe_society.is_some() 
				&& maybe_society.clone().unwrap()
					.members.contains(&who), 
			Error::<T>::NotMember
		);
		Ok(maybe_society.unwrap())
	}

	fn check_society_phase(
		society_id: SocietyId,
		phase: Phase, 
	) -> DispatchResult {
		let mut society_phase = SocietyStatus::<T>::get(society_id.clone());
		// TODO: error handling
		let current_phase = society_phase.pop().unwrap().1;
		match phase {
			Phase::Commit => 
				ensure!(
					current_phase.eq(&Phase::Commit),
					Error::<T>::NotCommitPhase
				),
			Phase::Join => 
				ensure!(
					current_phase.eq(&Phase::Join), 
					Error::<T>::NotJoinPhase
				),
			_ => ()
		}
		Ok(())
	}

	/// try to update the phase of the society to 'Join', allowing
	/// participants to submit public keys
	/// will fail if there is less than a threshold of commitments submitted
	/// 
	/// * `block_number`: the current block number (should be a deadline)
	/// * `society_id`: the society to whose status should be updated
	/// 
	fn try_set_join(
		block_number: T::BlockNumber,
		society_id: SocietyId,
	) -> DispatchResult {
		// Error handling: assumes the society exists
		let society = Societies::<T>::get(society_id.clone()).unwrap();
		// check if there's a threshold of commitments
		let threshold = society.clone().threshold;
		let size = society.clone().members.len() as u8;
		let min_shares = threshold * size;
		let shares = SharesAndCommitments::<T>::get(society_id.clone());
		ensure!(
			shares.len() >= min_shares.into(),
			Error::<T>::ThresholdNotReached,
		);

		SocietyStatus::<T>::mutate(society_id.clone(), |v| {
			v.push((block_number, Phase::Join));
		});
		Ok(())
	}

	/// try to update the phase of the society to 'Active', essentially
	/// finalizing valid participants
	/// will fail if there is less than a threshold of commitments submitted
	/// 
	/// * `block_number`: the current block number (should be a deadline)
	/// * `society_id`: the society to whose status should be updated
	/// 
	fn try_set_commit(
		block_number: T::BlockNumber,
		society_id: SocietyId,
	) -> DispatchResult {
		// Error handling: assumes the society exists
		let society = Societies::<T>::get(society_id.clone()).unwrap();
		// check if there's a threshold of public keys
		let threshold = society.clone().threshold;
		let size = society.clone().members.len() as u8;
		let min_shares = threshold * size;
		let pubkeys = RSVP::<T>::get(society_id.clone());
		// TODO: verify public keys?
		ensure!(
			pubkeys.len() >= min_shares.into(),
			Error::<T>::ThresholdNotReached,
		);

		SocietyStatus::<T>::mutate(society_id.clone(), |v| {
			v.push((block_number, Phase::Active));
		});
		Ok(())
	}
}