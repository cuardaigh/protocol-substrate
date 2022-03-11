use core::fmt::Debug;
use subxt::{DefaultConfig, Event, TransactionProgress};

use webb_client::webb_runtime;
use webb_runtime::runtime_types::{sp_runtime::DispatchError, webb_standalone_runtime::Element};

use ark_ff::{BigInteger, PrimeField};

use arkworks_setups::common::Leaf;
use arkworks_setups::AnchorProver;
use arkworks_setups::r1cs::anchor::AnchorR1CSProver;
use arkworks_setups::MixerProver;
use arkworks_setups::r1cs::mixer::MixerR1CSProver;
pub use arkworks_setups::common::{prove, prove_unchecked, verify_unchecked_raw};
use arkworks_setups::Curve;

// wasm-utils dependencies
use ark_std::{rand::thread_rng, UniformRand};
use wasm_utils::{
	proof::{generate_proof_js, AnchorProofInput, JsProofInput, MixerProofInput, ProofInput},
	types::{Backend, Curve as WasmCurve},
};

use ark_bn254::Fr as Bn254Fr;
use ark_bn254::Bn254;

const TREE_DEPTH: usize = 30;
const ANCHOR_CT: usize = 2;
type AnchorProver_Bn254_30_2 = AnchorR1CSProver<Bn254, TREE_DEPTH, ANCHOR_CT>;
type MixerProver_Bn254_30 = MixerR1CSProver<Bn254, TREE_DEPTH>;

pub fn setup_mixer_leaf() -> (Element, Element, Element, Element) {
	let rng = &mut thread_rng();
	let curve = Curve::Bn254;
	let secret = Bn254Fr::rand(rng).into_repr().to_bytes_le();
	let nullifier = Bn254Fr::rand(rng).into_repr().to_bytes_le();
	let Leaf { secret_bytes, nullifier_bytes, leaf_bytes, nullifier_hash_bytes, .. } =
		MixerProver_Bn254_30::create_leaf_with_privates(curve, secret, nullifier).unwrap();

	let leaf_array: [u8; 32] = leaf_bytes.try_into().unwrap();
	let leaf_element = Element(leaf_array);

	let secret_array: [u8; 32] = secret_bytes.try_into().unwrap();
	let secret_element = Element(secret_array);

	let nullifier_array: [u8; 32] = nullifier_bytes.try_into().unwrap();
	let nullifier_element = Element(nullifier_array);

	let nullifier_hash_array: [u8; 32] = nullifier_hash_bytes.try_into().unwrap();
	let nullifier_hash_element = Element(nullifier_hash_array);

	(leaf_element, secret_element, nullifier_element, nullifier_hash_element)
}

pub fn setup_anchor_leaf(chain_id: u64) -> (Element, Element, Element, Element) {
	let rng = &mut thread_rng();
	let curve = Curve::Bn254;
	let secret = Bn254Fr::rand(rng).into_repr().to_bytes_le();
	let nullifier = Bn254Fr::rand(rng).into_repr().to_bytes_le();
	let Leaf { secret_bytes, nullifier_bytes, leaf_bytes, nullifier_hash_bytes, .. } =
		AnchorProver_Bn254_30_2::create_leaf_with_privates(curve, chain_id, secret, nullifier).unwrap();

	let leaf_array: [u8; 32] = leaf_bytes.try_into().unwrap();
	let leaf_element = Element(leaf_array);

	let secret_array: [u8; 32] = secret_bytes.try_into().unwrap();
	let secret_element = Element(secret_array);

	let nullifier_array: [u8; 32] = nullifier_bytes.try_into().unwrap();
	let nullifier_element = Element(nullifier_array);

	let nullifier_hash_array: [u8; 32] = nullifier_hash_bytes.try_into().unwrap();
	let nullifier_hash_element = Element(nullifier_hash_array);

	(leaf_element, secret_element, nullifier_element, nullifier_hash_element)
}

pub fn setup_mixer_circuit(
	leaves: Vec<Vec<u8>>,
	leaf_index: u64,
	secret: Vec<u8>,
	nullifier: Vec<u8>,
	recipient_bytes: Vec<u8>,
	relayer_bytes: Vec<u8>,
	fee_value: u128,
	refund_value: u128,
	pk_bytes: Vec<u8>,
) -> (
	Vec<u8>, // proof bytes
	Element, // root
) {
	let mixer_proof_input = MixerProofInput {
		exponentiation: 5,
		width: 5,
		curve: WasmCurve::Bn254,
		backend: Backend::Arkworks,
		secret,
		nullifier,
		recipient: recipient_bytes,
		relayer: relayer_bytes,
		pk: pk_bytes,
		refund: refund_value,
		fee: fee_value,
		chain_id: 0,
		leaves,
		leaf_index,
	};
	let js_proof_inputs = JsProofInput { inner: ProofInput::Mixer(mixer_proof_input) };
	let proof = generate_proof_js(js_proof_inputs).unwrap();

	let root_array: [u8; 32] = proof.root.try_into().unwrap();
	let root_element = Element(root_array);

	(proof.proof, root_element)
}

pub fn setup_anchor_circuit(
	roots: Vec<Vec<u8>>,
	leaves: Vec<Vec<u8>>,
	leaf_index: u64,
	chain_id: u64,
	secret: Vec<u8>,
	nullifier: Vec<u8>,
	recipient_bytes: Vec<u8>,
	relayer_bytes: Vec<u8>,
	fee_value: u128,
	refund_value: u128,
	commitment_bytes: Vec<u8>,
	pk_bytes: Vec<u8>,
) -> (
	Vec<u8>, // proof bytes
	Element, // root
) {
	let commitment: [u8; 32] = commitment_bytes.try_into().unwrap();
	let anchor_proof_input = AnchorProofInput {
		exponentiation: 5,
		width: 4,
		curve: WasmCurve::Bn254,
		backend: Backend::Arkworks,
		secret,
		nullifier,
		recipient: recipient_bytes,
		relayer: relayer_bytes,
		pk: pk_bytes,
		refund: refund_value,
		fee: fee_value,
		chain_id,
		leaves,
		leaf_index,
		roots,
		refresh_commitment: commitment,
	};
	let js_proof_inputs = JsProofInput { inner: ProofInput::Anchor(anchor_proof_input) };
	let proof = generate_proof_js(js_proof_inputs).unwrap();

	let root_array: [u8; 32] = proof.root.try_into().unwrap();
	let root_element = Element(root_array);

	(proof.proof, root_element)
}

pub async fn expect_event<E: Event + Debug>(
	tx_progess: &mut TransactionProgress<'_, DefaultConfig, DispatchError>,
) -> Result<(), Box<dyn std::error::Error>> {
	// Start printing on a fresh line
	println!("");

	while let Some(ev) = tx_progess.next_item().await {
		let ev = ev?;
		use subxt::TransactionStatus::*;

		// Made it into a block, but not finalized.
		if let InBlock(details) = ev {
			println!(
				"Transaction {:?} made it into block {:?}",
				details.extrinsic_hash(),
				details.block_hash()
			);

			let events = details.wait_for_success().await?;
			let transfer_event = events.find_first_event::<E>()?;

			if let Some(event) = transfer_event {
				println!("In block (but not finalized): {event:?}");
			} else {
				println!("Failed to find Event");
			}
		}
		// Finalized!
		else if let Finalized(details) = ev {
			println!(
				"Transaction {:?} is finalized in block {:?}",
				details.extrinsic_hash(),
				details.block_hash()
			);

			let events = details.wait_for_success().await?;
			let transfer_event = events.find_first_event::<E>()?;

			if let Some(event) = transfer_event {
				println!("Transaction success: {event:?}");
			} else {
				println!("Failed to find Balances::Transfer Event");
			}
		}
		// Report other statuses we see.
		else {
			println!("Current transaction status: {:?}", ev);
		}
	}

	Ok(())
}

pub fn truncate_and_pad(t: &[u8]) -> Vec<u8> {
	let mut truncated_bytes = t[..20].to_vec();
	truncated_bytes.extend_from_slice(&[0u8; 12]);
	truncated_bytes
}
