use katana_provider::traits::state::StateProvider;

pub struct Context<P>
where
    P: StateProvider,
{
    // context id
    id: u64,
    // state provider
    provider: P,
}
