

    use katana_core::{
        backend::config::StarknetConfig,
        constants::{DEFAULT_INVOKE_MAX_STEPS, DEFAULT_VALIDATE_MAX_STEPS, MAX_RECURSION_DEPTH},
        env::get_default_vm_resource_fee_cost,
    };
    use katana_executor::implementation::blockifier::BlockifierFactory;
    use katana_primitives::felt::FieldElement;
    use katana_primitives::{
        chain::ChainId,
        env::{CfgEnv, FeeTokenAddressses},
        genesis::constant::DEFAULT_FEE_TOKEN_ADDRESS,
        transaction::{InvokeTx, InvokeTxV1},
    };
    use starknet::macros::{felt, selector};
    use tracing_subscriber::{fmt, EnvFilter};

    fn tx() -> ExecutableTxWithHash {
        let invoke = InvokeTx::V1(InvokeTxV1 {
            sender_address: felt!("0x1").into(),
            calldata: vec![
                DEFAULT_FEE_TOKEN_ADDRESS.into(),
                selector!("transfer"),
                FieldElement::THREE,
                felt!("0x100"),
                FieldElement::ONE,
                FieldElement::ZERO,
            ],
            max_fee: 10_000,
            ..Default::default()
        });

        ExecutableTxWithHash::new(invoke.into())
    }

    async fn create_tenants_backend() -> HashMap<u8, Arc<Backend<BlockifierFactory>>> {
        let mut backends = HashMap::new();

        let chains = [
            ChainId::MAINNET,
            ChainId::SEPOLIA,
            ChainId::GOERLI,
            ChainId::parse("KATANA").unwrap(),
        ];

        for i in 0..4 {
            let cfg_env = CfgEnv {
                chain_id: chains[i],
                vm_resource_fee_cost: get_default_vm_resource_fee_cost(),
                invoke_tx_max_n_steps: DEFAULT_INVOKE_MAX_STEPS,
                validate_max_n_steps: DEFAULT_VALIDATE_MAX_STEPS,
                max_recursion_depth: MAX_RECURSION_DEPTH,
                fee_token_addresses: FeeTokenAddressses {
                    eth: DEFAULT_FEE_TOKEN_ADDRESS,
                    strk: Default::default(),
                },
            };

            let mut starknet_config = StarknetConfig::default();
            starknet_config.db_dir = Some(format!("./db/{i}").into());

            let flag = SimulationFlag {
                skip_fee_transfer: true,
                skip_validate: true,
                ..Default::default()
            };

            let executor_factory = BlockifierFactory::new(cfg_env, flag);
            let backend = Backend::new(Arc::new(executor_factory), starknet_config).await;
            backends.insert(i as u8, Arc::new(backend));
        }

        backends
    }

    const DEFAULT_LOG_FILTER: &str = "info,executor=trace,forking::backend=trace,server=debug,\
                                      katana_core=trace,blockifier=off,jsonrpsee_server=off,\
                                      hyper=off,messaging=debug,node=error";

    #[tokio::main]
    async fn main() -> anyhow::Result<()> {
        let subscriber = fmt::Subscriber::builder()
            .with_env_filter(
                EnvFilter::try_from_default_env().or(EnvFilter::try_new(DEFAULT_LOG_FILTER))?,
            )
            .finish();

        tracing::subscriber::set_global_default(subscriber)?;

        // Configure the runtiem with the specified number of worker threads and tenants
        // This basically means that the runtime will have 4 worker threads and will be able to handle requests from 4 different tenants
        // in parallel.
        let backends = create_tenants_backend().await;
        let runtime = Runtime::new(4, backends);

        let handle = runtime.spawn(AddTask {
            tenant_id: 0,
            transactions: vec![tx()],
        });
        // let _ = runtime.spawn(AddTask::default());
        // let _ = runtime.spawn(AddTask::default());

        let result = handle.await;

        println!("Result: {result:?}");

        Ok(())
    }
