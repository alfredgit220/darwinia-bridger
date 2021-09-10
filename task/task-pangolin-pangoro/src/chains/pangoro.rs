pub use s2s_const::*;
pub use s2s_headers::*;
pub use s2s_messages::*;

mod s2s_const {
    use std::time::Duration;

    use bp_messages::MessageNonce;
    use bp_runtime::ChainId;
    use frame_support::weights::Weight;
    use relay_substrate_client::Chain;
    use sp_version::RuntimeVersion;

    use component_pangolin_s2s::PangolinChain;
    use component_pangoro_s2s::PangoroChain;

    use crate::traits::{ChainConst, CliChain};

    // === start const
    impl CliChain for PangoroChain {
        const RUNTIME_VERSION: RuntimeVersion = pangoro_runtime::VERSION;

        type KeyPair = sp_core::sr25519::Pair;
    }

    pub struct PangoroChainConst;

    impl ChainConst for PangoroChainConst {
        const OUTBOUND_LANE_MESSAGE_DETAILS_METHOD: &'static str =
            bridge_primitives::TO_PANGORO_MESSAGE_DETAILS_METHOD;
        const OUTBOUND_LANE_LATEST_GENERATED_NONCE_METHOD: &'static str =
            bridge_primitives::TO_PANGORO_LATEST_GENERATED_NONCE_METHOD;
        const OUTBOUND_LANE_LATEST_RECEIVED_NONCE_METHOD: &'static str =
            bridge_primitives::TO_PANGORO_LATEST_RECEIVED_NONCE_METHOD;
        const INBOUND_LANE_LATEST_RECEIVED_NONCE_METHOD: &'static str =
            bridge_primitives::FROM_PANGORO_LATEST_RECEIVED_NONCE_METHOD;
        const INBOUND_LANE_LATEST_CONFIRMED_NONCE_METHOD: &'static str =
            bridge_primitives::FROM_PANGORO_LATEST_CONFIRMED_NONCE_METHOD;
        const INBOUND_LANE_UNREWARDED_RELAYERS_STATE: &'static str =
            bridge_primitives::FROM_PANGORO_UNREWARDED_RELAYERS_STATE;
        const BEST_FINALIZED_SOURCE_HEADER_ID_AT_TARGET: &'static str =
            bridge_primitives::BEST_FINALIZED_PANGORO_HEADER_METHOD;
        const BEST_FINALIZED_TARGET_HEADER_ID_AT_SOURCE: &'static str =
            bridge_primitives::BEST_FINALIZED_PANGORO_HEADER_METHOD;
        const MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE: MessageNonce =
            bridge_primitives::MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE;
        const MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE: MessageNonce =
            bridge_primitives::MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE;
        const AVERAGE_BLOCK_INTERVAL: Duration = PangolinChain::AVERAGE_BLOCK_INTERVAL;
        const BRIDGE_CHAIN_ID: ChainId = bridge_primitives::PANGORO_CHAIN_ID;
        const MESSAGE_PALLET_NAME_AT_SOURCE: &'static str =
            bridge_primitives::WITH_PANGORO_MESSAGES_PALLET_NAME;
        const MESSAGE_PALLET_NAME_AT_TARGET: &'static str =
            bridge_primitives::WITH_PANGOLIN_MESSAGES_PALLET_NAME;
        const PAY_INBOUND_DISPATCH_FEE_WEIGHT_AT_TARGET_CHAIN: Weight =
            bridge_primitives::PAY_INBOUND_DISPATCH_FEE_WEIGHT;
        type SigningParams = common_primitives::SigningParams;
    }

    // === end
}

mod s2s_headers {
    use bp_header_chain::justification::GrandpaJustification;
    use codec::Encode;
    use relay_substrate_client::{Chain, Client, TransactionSignScheme};
    use sp_core::{Bytes, Pair};
    use substrate_relay_helper::finality_pipeline::{
        SubstrateFinalitySyncPipeline, SubstrateFinalityToSubstrate,
    };

    use component_pangolin_s2s::PangolinChain;
    use component_pangoro_s2s::PangoroChain;

    use crate::chains::pangolin::PangolinChainConst;
    use crate::chains::pangoro::PangoroChainConst;
    use crate::traits::ChainConst;

    // === start pangolin headers to pangoro
    /// Pangoro-to-Pangolin finality sync pipeline.
    pub(crate) type FinalityPipelinePangoroFinalityToPangolin = SubstrateFinalityToSubstrate<
        PangoroChain,
        PangolinChain,
        <PangolinChainConst as ChainConst>::SigningParams,
    >;

    #[derive(Clone, Debug)]
    pub struct PangoroFinalityToPangolin {
        finality_pipeline: FinalityPipelinePangoroFinalityToPangolin,
    }

    impl PangoroFinalityToPangolin {
        pub fn new(
            target_client: Client<PangolinChain>,
            target_sign: <PangolinChainConst as ChainConst>::SigningParams,
        ) -> Self {
            Self {
                finality_pipeline: FinalityPipelinePangoroFinalityToPangolin::new(
                    target_client,
                    target_sign,
                ),
            }
        }
    }

    impl SubstrateFinalitySyncPipeline for PangoroFinalityToPangolin {
        type FinalitySyncPipeline = FinalityPipelinePangoroFinalityToPangolin;

        const BEST_FINALIZED_SOURCE_HEADER_ID_AT_TARGET: &'static str =
            PangoroChainConst::BEST_FINALIZED_SOURCE_HEADER_ID_AT_TARGET;

        type TargetChain = PangolinChain;

        fn transactions_author(&self) -> common_primitives::AccountId {
            (*self.finality_pipeline.target_sign.public().as_array_ref()).into()
        }

        fn make_submit_finality_proof_transaction(
            &self,
            era: bp_runtime::TransactionEraOf<PangolinChain>,
            transaction_nonce: <PangolinChain as Chain>::Index,
            header: component_pangoro_s2s::SyncHeader,
            proof: GrandpaJustification<common_primitives::Header>,
        ) -> Bytes {
            let call = pangolin_runtime::BridgeGrandpaCall::<
                pangolin_runtime::Runtime,
                pangolin_runtime::WithPangoroGrandpa,
            >::submit_finality_proof(header.into_inner(), proof)
            .into();

            let genesis_hash = *self.finality_pipeline.target_client.genesis_hash();
            let transaction = PangolinChain::sign_transaction(
                genesis_hash,
                &self.finality_pipeline.target_sign,
                era,
                transaction_nonce,
                call,
            );

            Bytes(transaction.encode())
        }
    }

    // === end
}

mod s2s_messages {
    use std::{ops::RangeInclusive, time::Duration};

    use bp_messages::MessageNonce;
    use bridge_runtime_common::messages::target::FromBridgedChainMessagesProof;
    use codec::Encode;
    use frame_support::dispatch::GetDispatchInfo;
    use frame_support::weights::Weight;
    use messages_relay::message_lane::MessageLane;
    use relay_substrate_client::{Client, TransactionSignScheme};
    use relay_utils::metrics::MetricsParams;
    use sp_core::{Bytes, Pair};
    use substrate_relay_helper::messages_lane::{
        MessagesRelayParams, StandaloneMessagesMetrics, SubstrateMessageLane,
        SubstrateMessageLaneToSubstrate,
    };
    use substrate_relay_helper::messages_source::SubstrateMessagesSource;
    use substrate_relay_helper::messages_target::SubstrateMessagesTarget;

    use component_pangolin_s2s::PangolinChain;
    use component_pangoro_s2s::PangoroChain;

    use crate::chains::pangolin::PangolinChainConst;
    use crate::chains::pangoro::PangoroChainConst;
    use crate::traits::ChainConst;

    pub const SOURCE_NAME: &str = "pangoro";
    pub const TARGET_NAME: &str = "pangolin";

    /// Source-to-Target message lane.
    pub type MessageLanePangoroMessagesToPangolin = SubstrateMessageLaneToSubstrate<
        PangoroChain,
        <PangoroChainConst as ChainConst>::SigningParams,
        PangolinChain,
        <PangolinChainConst as ChainConst>::SigningParams,
    >;

    #[derive(Clone)]
    pub struct PangoroMessagesToPangolin {
        message_lane: MessageLanePangoroMessagesToPangolin,
    }

    impl SubstrateMessageLane for PangoroMessagesToPangolin {
        type MessageLane = MessageLanePangoroMessagesToPangolin;

        const OUTBOUND_LANE_MESSAGE_DETAILS_METHOD: &'static str =
            PangolinChainConst::OUTBOUND_LANE_MESSAGE_DETAILS_METHOD;
        const OUTBOUND_LANE_LATEST_GENERATED_NONCE_METHOD: &'static str =
            PangolinChainConst::OUTBOUND_LANE_LATEST_GENERATED_NONCE_METHOD;
        const OUTBOUND_LANE_LATEST_RECEIVED_NONCE_METHOD: &'static str =
            PangolinChainConst::OUTBOUND_LANE_LATEST_RECEIVED_NONCE_METHOD;

        const INBOUND_LANE_LATEST_RECEIVED_NONCE_METHOD: &'static str =
            PangoroChainConst::INBOUND_LANE_LATEST_RECEIVED_NONCE_METHOD;
        const INBOUND_LANE_LATEST_CONFIRMED_NONCE_METHOD: &'static str =
            PangoroChainConst::INBOUND_LANE_LATEST_CONFIRMED_NONCE_METHOD;
        const INBOUND_LANE_UNREWARDED_RELAYERS_STATE: &'static str =
            PangoroChainConst::INBOUND_LANE_UNREWARDED_RELAYERS_STATE;

        const BEST_FINALIZED_SOURCE_HEADER_ID_AT_TARGET: &'static str =
            PangoroChainConst::BEST_FINALIZED_SOURCE_HEADER_ID_AT_TARGET;
        const BEST_FINALIZED_TARGET_HEADER_ID_AT_SOURCE: &'static str =
            PangolinChainConst::BEST_FINALIZED_TARGET_HEADER_ID_AT_SOURCE;

        const MESSAGE_PALLET_NAME_AT_SOURCE: &'static str =
            PangoroChainConst::MESSAGE_PALLET_NAME_AT_SOURCE;
        const MESSAGE_PALLET_NAME_AT_TARGET: &'static str =
            PangoroChainConst::MESSAGE_PALLET_NAME_AT_TARGET;
        const PAY_INBOUND_DISPATCH_FEE_WEIGHT_AT_TARGET_CHAIN: Weight =
            PangoroChainConst::PAY_INBOUND_DISPATCH_FEE_WEIGHT_AT_TARGET_CHAIN;

        type SourceChain = PangoroChain;
        type TargetChain = PangolinChain;

        fn source_transactions_author(&self) -> common_primitives::AccountId {
            (*self.message_lane.source_sign.public().as_array_ref()).into()
        }

        fn make_messages_receiving_proof_transaction(
            &self,
            transaction_nonce: <PangoroChain as relay_substrate_client::Chain>::Index,
            _generated_at_block: component_pangolin_s2s::HeaderId,
            proof: <Self::MessageLane as MessageLane>::MessagesReceivingProof,
        ) -> Bytes {
            let (relayers_state, proof) = proof;
            let call: pangoro_runtime::Call =
                pangoro_runtime::BridgeMessagesCall::receive_messages_delivery_proof::<
                    pangoro_runtime::Runtime,
                    pangoro_runtime::WithPangolinMessages,
                >(proof, relayers_state)
                .into();
            let call_weight = call.get_dispatch_info().weight;
            let genesis_hash = *self.message_lane.source_client.genesis_hash();
            let transaction = PangoroChain::sign_transaction(
                genesis_hash,
                &self.message_lane.source_sign,
                relay_substrate_client::TransactionEra::immortal(),
                transaction_nonce,
                call,
            );
            log::trace!(
                target: "bridge",
                "Prepared {} -> {} confirmation transaction. Weight: {}/{}, size: {}/{}",
                TARGET_NAME,
                SOURCE_NAME,
                call_weight,
                pangoro_runtime_system_params::max_extrinsic_weight(),
                transaction.encode().len(),
                pangoro_runtime_system_params::max_extrinsic_size(),
            );
            Bytes(transaction.encode())
        }

        fn target_transactions_author(&self) -> common_primitives::AccountId {
            (*self.message_lane.target_sign.public().as_array_ref()).into()
        }

        fn make_messages_delivery_transaction(
            &self,
            transaction_nonce: <PangolinChain as relay_substrate_client::Chain>::Index,
            _generated_at_header: component_pangoro_s2s::HeaderId,
            _nonces: RangeInclusive<MessageNonce>,
            proof: <Self::MessageLane as MessageLane>::MessagesProof,
        ) -> Bytes {
            let (dispatch_weight, proof) = proof;
            let FromBridgedChainMessagesProof {
                ref nonces_start,
                ref nonces_end,
                ..
            } = proof;
            let messages_count = nonces_end - nonces_start + 1;
            let call: pangolin_runtime::Call =
                pangolin_runtime::BridgeMessagesCall::receive_messages_proof::<
                    pangolin_runtime::Runtime,
                    pangolin_runtime::WithPangoroMessages,
                >(
                    self.message_lane.relayer_id_at_source.clone(),
                    proof,
                    messages_count as _,
                    dispatch_weight,
                )
                .into();
            let call_weight = call.get_dispatch_info().weight;
            let genesis_hash = *self.message_lane.target_client.genesis_hash();
            let transaction = PangolinChain::sign_transaction(
                genesis_hash,
                &self.message_lane.target_sign,
                relay_substrate_client::TransactionEra::immortal(),
                transaction_nonce,
                call,
            );
            log::trace!(
                target: "bridge",
                "Prepared {} -> {} delivery transaction. Weight: {}/{}, size: {}/{}",
                SOURCE_NAME,
                TARGET_NAME,
                call_weight,
                pangoro_runtime_system_params::max_extrinsic_weight(),
                transaction.encode().len(),
                pangolin_runtime_system_params::max_extrinsic_size(),
            );
            Bytes(transaction.encode())
        }
    }

    /// Source node as messages source.
    type PangoroSourceClient = SubstrateMessagesSource<PangoroMessagesToPangolin>;

    /// Target node as messages target.
    type PangolinTargetClient = SubstrateMessagesTarget<PangoroMessagesToPangolin>;

    pub struct PangoroMessagesToPangolinRunner;

    #[allow(non_snake_case)]
    impl PangoroMessagesToPangolinRunner {
        pub async fn run(
            params: MessagesRelayParams<
                PangoroChain,
                <PangoroChainConst as ChainConst>::SigningParams,
                PangolinChain,
                <PangolinChainConst as ChainConst>::SigningParams,
            >,
        ) -> anyhow::Result<()> {
            let stall_timeout = Duration::from_secs(5 * 60);
            let relayer_id_at_Pangoro = (*params.source_sign.public().as_array_ref()).into();

            let lane_id = params.lane_id;
            let source_client = params.source_client;
            let lane = PangoroMessagesToPangolin {
                message_lane: SubstrateMessageLaneToSubstrate {
                    source_client: source_client.clone(),
                    source_sign: params.source_sign,
                    target_client: params.target_client.clone(),
                    target_sign: params.target_sign,
                    relayer_id_at_source: relayer_id_at_Pangoro,
                },
            };

            // 2/3 is reserved for proofs and tx overhead
            let max_messages_size_in_single_batch =
                pangolin_runtime_system_params::max_extrinsic_size() / 3;
            let (max_messages_in_single_batch, max_messages_weight_in_single_batch) =
                substrate_relay_helper::messages_lane::select_delivery_transaction_limits::<
                    // todo: there can be change to special weight
                    pallet_bridge_messages::weights::RialtoWeight<pangoro_runtime::Runtime>,
                >(
                    pangolin_runtime_system_params::max_extrinsic_weight(),
                    PangolinChainConst::MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE,
                );

            log::info!(
                target: "bridge",
                "Starting {} -> {} messages relay.\n\t\
                {} relayer account id: {:?}\n\t\
                Max messages in single transaction: {}\n\t\
                Max messages size in single transaction: {}\n\t\
                Max messages weight in single transaction: {}",
                SOURCE_NAME,
                TARGET_NAME,
                SOURCE_NAME,
                lane.message_lane.relayer_id_at_source,
                max_messages_in_single_batch,
                max_messages_size_in_single_batch,
                max_messages_weight_in_single_batch,
            );

            let (metrics_params, metrics_values) = add_standalone_metrics(
                Some(messages_relay::message_lane_loop::metrics_prefix::<
                    <PangoroMessagesToPangolin as SubstrateMessageLane>::MessageLane,
                >(&lane_id)),
                params.metrics_params,
                source_client.clone(),
            )?;
            messages_relay::message_lane_loop::run(
                messages_relay::message_lane_loop::Params {
                    lane: lane_id,
                    source_tick: PangoroChainConst::AVERAGE_BLOCK_INTERVAL,
                    target_tick: PangolinChainConst::AVERAGE_BLOCK_INTERVAL,
                    reconnect_delay: relay_utils::relay_loop::RECONNECT_DELAY,
                    stall_timeout,
                    delivery_params: messages_relay::message_lane_loop::MessageDeliveryParams {
                        max_unrewarded_relayer_entries_at_target:
                            PangolinChainConst::MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE,
                        max_unconfirmed_nonces_at_target:
                            PangolinChainConst::MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE,
                        max_messages_in_single_batch,
                        max_messages_weight_in_single_batch,
                        max_messages_size_in_single_batch,
                        relayer_mode: params.relayer_mode,
                    },
                },
                PangoroSourceClient::new(
                    source_client.clone(),
                    lane.clone(),
                    lane_id,
                    params.target_to_source_headers_relay,
                ),
                PangolinTargetClient::new(
                    params.target_client,
                    lane,
                    lane_id,
                    metrics_values,
                    params.source_to_target_headers_relay,
                ),
                metrics_params,
                futures::future::pending(),
            )
            .await
        }
    }

    /// Add standalone metrics for the Pangoro -> Pangolin messages loop.
    pub(crate) fn add_standalone_metrics(
        metrics_prefix: Option<String>,
        metrics_params: MetricsParams,
        source_client: Client<PangoroChain>,
    ) -> anyhow::Result<(MetricsParams, StandaloneMessagesMetrics)> {
        substrate_relay_helper::messages_lane::add_standalone_metrics::<PangoroMessagesToPangolin>(
            metrics_prefix,
            metrics_params,
            source_client,
            Some(crate::chains::PANGORO_ASSOCIATED_TOKEN_ID),
            Some(crate::chains::PANGOLIN_ASSOCIATED_TOKEN_ID),
            Some((
                sp_core::storage::StorageKey(
                    pangoro_runtime::pangolin_messages::PangolinToPangoroConversionRate::key()
                        .to_vec(),
                ),
                pangoro_runtime::pangolin_messages::INITIAL_PANGOLIN_TO_PANGORO_CONVERSION_RATE,
            )),
        )
    }
}
