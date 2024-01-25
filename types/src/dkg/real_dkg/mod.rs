// Copyright © Aptos Foundation

use crate::{
    dkg::{real_dkg::rounding::DKGRounding, DKGSessionMetadata, DKGTrait},
    validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
};
use anyhow::ensure;
use aptos_crypto::bls12381;
use aptos_dkg::{
    pvss,
    pvss::{
        traits::{Convert, Reconstructable, Transcript},
        Player,
    },
};
use num_traits::Zero;
use rand::{CryptoRng, RngCore};
use rounding::{
    RECONSTRUCT_THRESHOLD, STAKE_GAP_THRESHOLD, STEPS, WEIGHT_PER_VALIDATOR_MAX,
    WEIGHT_PER_VALIDATOR_MIN,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub mod rounding;

pub type WTrx = pvss::das::WeightedTranscript;
pub type DkgPP = <WTrx as Transcript>::PublicParameters;
pub type SSConfig = <WTrx as Transcript>::SecretSharingConfig;
pub type EncPK = <WTrx as Transcript>::EncryptPubKey;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct DKGPvssConfig {
    pub epoch: u64,
    // weighted config for randomness generation
    pub wconfig: SSConfig,
    // DKG public parameters
    pub pp: DkgPP,
    // DKG encryption public keys
    pub eks: Vec<EncPK>,
}

impl DKGPvssConfig {
    pub fn new(epoch: u64, wconfig: SSConfig, pp: DkgPP, eks: Vec<EncPK>) -> Self {
        Self {
            epoch,
            wconfig,
            pp,
            eks,
        }
    }
}

pub fn build_dkg_pvss_config(
    cur_epoch: u64,
    next_validators: &[ValidatorConsensusInfo],
) -> DKGPvssConfig {
    let validator_stakes: Vec<u64> = next_validators.iter().map(|vi| vi.voting_power).collect();

    // // For mainnet-like testing
    // let validator_stakes: Vec<u64> = MAINNET_STAKES.to_vec();
    // assert!(validator_stakes.len() == next_validator_set.active_validators.len());

    let dkg_rounding = DKGRounding::new(
        validator_stakes.clone(),
        STAKE_GAP_THRESHOLD,
        WEIGHT_PER_VALIDATOR_MIN,
        WEIGHT_PER_VALIDATOR_MAX,
        STEPS,
        RECONSTRUCT_THRESHOLD,
    );

    let validator_consensus_keys: Vec<bls12381::PublicKey> = next_validators
        .iter()
        .map(|vi| vi.public_key.clone())
        .collect();

    let consensus_keys: Vec<EncPK> = validator_consensus_keys
        .iter()
        .map(|k| k.to_bytes().as_slice().try_into().unwrap())
        .collect::<Vec<_>>();

    let wconfig = dkg_rounding.wconfig.clone();

    let pp = DkgPP::default_with_bls_base();

    DKGPvssConfig::new(cur_epoch, wconfig.clone(), pp, consensus_keys)
}

#[derive(Debug)]
pub struct RealDKG {}

#[derive(Clone, Debug)]
pub struct RealDKGPublicParams {
    pub session_metadata: DKGSessionMetadata,
    pub pvss_config: DKGPvssConfig,
    pub verifier: ValidatorVerifier,
}

impl DKGTrait for RealDKG {
    type DealerPrivateKey = <WTrx as Transcript>::SigningSecretKey;
    type DealtPubKeyShare = <WTrx as Transcript>::DealtPubKeyShare;
    type DealtSecret = <WTrx as Transcript>::DealtSecretKey;
    type DealtSecretShare = <WTrx as Transcript>::DealtSecretKeyShare;
    type InputSecret = <WTrx as Transcript>::InputSecret;
    type NewValidatorDecryptKey = <WTrx as Transcript>::DecryptPrivKey;
    type PublicParams = RealDKGPublicParams;
    type Transcript = WTrx;

    fn new_public_params(dkg_session_metadata: &DKGSessionMetadata) -> RealDKGPublicParams {
        let pvss_config = build_dkg_pvss_config(
            dkg_session_metadata.dealer_epoch,
            &dkg_session_metadata.target_validator_consensus_infos_cloned(),
        );
        let verifier = ValidatorVerifier::new(dkg_session_metadata.dealer_consensus_infos_cloned());
        RealDKGPublicParams {
            session_metadata: dkg_session_metadata.clone(),
            pvss_config,
            verifier,
        }
    }

    fn aggregate_input_secret(secrets: Vec<Self::InputSecret>) -> Self::InputSecret {
        secrets
            .into_iter()
            .fold(<WTrx as Transcript>::InputSecret::zero(), |acc, item| {
                acc + item
            })
    }

    fn dealt_secret_from_input(
        pub_params: &Self::PublicParams,
        input: &Self::InputSecret,
    ) -> Self::DealtSecret {
        input.to(&pub_params.pvss_config.pp)
    }

    fn generate_transcript<R: CryptoRng + RngCore>(
        rng: &mut R,
        pub_params: &Self::PublicParams,
        input_secret: &Self::InputSecret,
        my_index: u64,
        sk: &Self::DealerPrivateKey,
    ) -> Self::Transcript {
        let my_index = my_index as usize;
        let my_addr = pub_params.session_metadata.dealer_validator_set[my_index].addr;
        let aux = (pub_params.session_metadata.dealer_epoch, my_addr);

        WTrx::deal(
            &pub_params.pvss_config.wconfig,
            &pub_params.pvss_config.pp,
            sk,
            &pub_params.pvss_config.eks,
            input_secret,
            &aux,
            &Player { id: my_index },
            rng,
        )
    }

    fn verify_transcript(
        params: &Self::PublicParams,
        trx: &Self::Transcript,
    ) -> anyhow::Result<()> {
        // Verify dealer indices are valid.
        let dealers = trx
            .get_dealers()
            .iter()
            .map(|player| player.id)
            .collect::<Vec<usize>>();
        let num_validators = params.session_metadata.dealer_validator_set.len();
        ensure!(
            dealers.iter().all(|id| *id < num_validators),
            "real_dkg::verify_transcript failed with invalid dealer index."
        );

        let all_eks = params.pvss_config.eks.clone();

        let addresses = params.verifier.get_ordered_account_addresses();
        let dealers_addresses = dealers
            .iter()
            .filter_map(|&pos| addresses.get(pos))
            .cloned()
            .collect::<Vec<_>>();

        let spks = dealers_addresses
            .iter()
            .filter_map(|author| params.verifier.get_public_key(author))
            .collect::<Vec<_>>();

        let aux = dealers_addresses
            .iter()
            .map(|address| (params.pvss_config.epoch, address))
            .collect::<Vec<_>>();

        trx.verify(
            &params.pvss_config.wconfig,
            &params.pvss_config.pp,
            &spks,
            &all_eks,
            &aux,
        )?;

        Ok(())
    }

    fn aggregate_transcripts(
        params: &Self::PublicParams,
        accumulator: &mut Self::Transcript,
        element: Self::Transcript,
    ) {
        accumulator.aggregate_with(&params.pvss_config.wconfig, &element);
    }

    fn decrypt_secret_share_from_transcript(
        pub_params: &Self::PublicParams,
        trx: &Self::Transcript,
        player_idx: u64,
        dk: &Self::NewValidatorDecryptKey,
    ) -> anyhow::Result<(Self::DealtSecretShare, Self::DealtPubKeyShare)> {
        let (sk, pk) = trx.decrypt_own_share(
            &pub_params.pvss_config.wconfig,
            &Player {
                id: player_idx as usize,
            },
            dk,
        );
        Ok((sk, pk))
    }

    fn reconstruct_secret_from_shares(
        pub_params: &Self::PublicParams,
        player_share_pairs: Vec<(u64, Self::DealtSecretShare)>,
    ) -> anyhow::Result<Self::DealtSecret> {
        let player_share_pairs = player_share_pairs
            .into_iter()
            .map(|(x, y)| (Player { id: x as usize }, y))
            .collect();
        let reconstructed_secret = <WTrx as Transcript>::DealtSecretKey::reconstruct(
            &pub_params.pvss_config.wconfig,
            &player_share_pairs,
        );
        Ok(reconstructed_secret)
    }

    fn get_dealers(transcript: &Self::Transcript) -> BTreeSet<u64> {
        transcript
            .get_dealers()
            .into_iter()
            .map(|x| x.id as u64)
            .collect()
    }
}
