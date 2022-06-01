use std::time::Duration;
use async_std::task;

use async_trait::async_trait;
use auto_impl::auto_impl;

// Note the order of the attributes here:
// `#[async_trait]` must appear first
#[async_trait]
#[auto_impl(&, Box, Arc)]
trait Component {
    async fn run(&self);
}

struct WaitABit(Duration);

#[async_trait]
impl Component for WaitABit {
    async fn run(&self) {
        task::sleep(self.0).await;
    }
}

async fn run_async(a: impl Component) {
    a.run().await;
}

#[async_std::main]
async fn main() {
    // We can treat our `Box<WaitABit>` as an `impl Component` directly
    run_async(Box::new(WaitABit(Duration::from_secs(1)))).await;
}
