use crate::lock::get_app_read;
use crate::core::project::{ProjectMetadata, transport::TransportState, generator::GeneratorInstance};

pub fn get_project_metadata<T, F>(mapper: F) -> anyhow::Result<T>
where F: Fn(&ProjectMetadata) -> T {
    let app = get_app_read();
    Ok(mapper(&app.metadata))
}

pub fn get_transport_state<T, F>(mapper: F) -> anyhow::Result<T>
where F: Fn(&TransportState) -> T {
    let app = get_app_read();
    Ok(mapper(&app.transport))
}

pub fn get_generator_list<C, U, M>(mapper: M) -> anyhow::Result<C>
where 
    M: Fn(u32, &GeneratorInstance) -> U, 
    C: FromIterator<U> 
{
    let app = get_app_read();
    Ok(app.generator_pool.iter().map(|(&id, gen)| mapper(id.to_u32(), gen.as_ref())).collect())
}

pub fn get_max_sample_index() -> anyhow::Result<u32> {
    let app = get_app_read();
    Ok(app.max_sample_index)
}