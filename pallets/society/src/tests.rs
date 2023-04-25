use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok};

#[test]
fn test_can_create_society() {
	new_test_ext().execute_with(|| {
		let id: crate::SocietyId = b"test".to_vec();
		let name: Vec<u8> = b"test".to_vec();
		let threshold = 2;
		let mut members = Vec::new();
		members.push(1);
		members.push(2);
		members.push(3);
		assert_ok!(Society::create(
			RuntimeOrigin::signed(1), 
			id.clone(),
			threshold.clone(),
			name.clone(),
			1,
			members.clone(),
		));
		let expected_society = crate::Society {
			founder: 1,
			members: members.clone(),
			threshold: threshold.clone(),
			name: name.clone()
		};
		// the society is created
		let society = crate::Societies::<Test>::get(id.clone()).unwrap();
		assert_eq!(expected_society, society);
		// the society is in the commit phase at block 0
		let phase = crate::SocietyStatus::<Test>::get(id.clone());
		assert_eq!(1, phase.len());
		assert_eq!(0, phase[0].0);
		assert_eq!(crate::Phase::Commit, phase[0].1);
		// a deadline has been recorded
		let deadline = crate::Deadlines::<Test>::get(1);
		assert_eq!(true, deadline.contains(&id.clone()));
		// each member has recieved an 'invitation'
		for member in members.clone().iter() {
			assert_eq!(crate::Membership::<Test>::get(
				member.clone(), crate::MemberStatus::Invitee,
			).contains(&id.clone()), true);
		}
	});
}

// #[test]
// fn test_create_fails_if_deadline_in_past() {

// }

// #[test]
// fn test_create_fails_when_society_id_taken() {

// }
// keeping for reference later
		// // Go past genesis block so events get deposited
		// System::set_block_number(1);
		// // Dispatch a signed extrinsic.
		// assert_ok!(TemplateModule::do_something(RuntimeOrigin::signed(1), 42));
		// // Read pallet storage and assert an expected result.
		// assert_eq!(TemplateModule::something(), Some(42));
		// // Assert that the correct event was deposited
		// System::assert_last_event(Event::SomethingStored { something: 42, who: 1 }.into());

#[test]
fn test_can_submit_commitments_when_invitee_and_commit_phase() {
	new_test_ext().execute_with(|| {
		let id: crate::SocietyId = b"test".to_vec();
		let name: Vec<u8> = b"test".to_vec();
		let threshold = 2;
		let mut members = Vec::new();
		members.push(1);
		members.push(2);
		members.push(3);
		assert_ok!(Society::create(
			RuntimeOrigin::signed(1), 
			id.clone(),
			threshold.clone(),
			name.clone(),
			1,
			members.clone(),
		));
		
		let mut mock_shares: Vec<crate::Share> = Vec::new();
		mock_shares.push(crate::Share {
			share: vec![1,2,3],
			commitment: vec![1,2,3],
		});
		mock_shares.push(crate::Share {
			share: vec![2, 3, 4],
			commitment: vec![2, 3, 4],
		});
		mock_shares.push(crate::Share {
			share: vec![3, 4, 5],
			commitment: vec![3, 4, 5],
		});
		assert_ok!(Society::commit(
			RuntimeOrigin::signed(2),
			id.clone(),
			mock_shares,
		));
		assert_eq!(crate::Membership::<Test>::get(
			2, crate::MemberStatus::Invitee,
		).contains(&id.clone()), false);
		assert_eq!(crate::Membership::<Test>::get(
			2, crate::MemberStatus::Committed,
		).contains(&id.clone()), true);
	});
}


#[test]
fn test_try_set_join_works_with_threshold_of_commitments() {
	new_test_ext().execute_with(|| {
		let id: crate::SocietyId = b"test".to_vec();
		let name: Vec<u8> = b"test".to_vec();
		let threshold = 2;
		let mut members = Vec::new();
		members.push(1);
		members.push(2);
		members.push(3);
		assert_ok!(Society::create(
			RuntimeOrigin::signed(1), 
			id.clone(),
			threshold.clone(),
			name.clone(),
			1,
			members.clone(),
		));

		let mut mock_shares: Vec<crate::Share> = Vec::new();
		mock_shares.push(crate::Share {
			share: vec![1,2,3],
			commitment: vec![1,2,3],
		});
		mock_shares.push(crate::Share {
			share: vec![2, 3, 4],
			commitment: vec![2, 3, 4],
		});
		mock_shares.push(crate::Share {
			share: vec![3, 4, 5],
			commitment: vec![3, 4, 5],
		});
		assert_ok!(Society::commit(
			RuntimeOrigin::signed(1),
			id.clone(),
			mock_shares.clone(),
		));
		assert_ok!(Society::commit(
			RuntimeOrigin::signed(2),
			id.clone(),
			mock_shares.clone(),
		));
		assert_ok!(Society::try_set_join(1, id.clone()));
		// update to 'Active' 
	});
}

#[test]
fn test_try_set_join_fails_with_sub_threshold_participants() {
	new_test_ext().execute_with(|| {
		let id: crate::SocietyId = b"test".to_vec();
		let name: Vec<u8> = b"test".to_vec();
		let threshold = 2;
		let mut members = Vec::new();
		members.push(1);
		members.push(2);
		members.push(3);
		assert_ok!(Society::create(
			RuntimeOrigin::signed(1), 
			id.clone(),
			threshold.clone(),
			name.clone(),
			1,
			members.clone(),
		));

		let mut mock_shares: Vec<crate::Share> = Vec::new();
		mock_shares.push(crate::Share {
			share: vec![1,2,3],
			commitment: vec![1,2,3],
		});
		mock_shares.push(crate::Share {
			share: vec![2, 3, 4],
			commitment: vec![2, 3, 4],
		});
		mock_shares.push(crate::Share {
			share: vec![3, 4, 5],
			commitment: vec![3, 4, 5],
		});
		assert_ok!(Society::commit(
			RuntimeOrigin::signed(1),
			id.clone(),
			mock_shares.clone(),
		));
		
		assert_noop!(
			Society::try_set_join(1, id.clone()), 
			Error::<Test>::ThresholdNotReached,
		);
	});
}