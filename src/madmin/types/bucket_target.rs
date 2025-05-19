#[derive(Debug, Clone, Default)]
pub struct BucketTargets {
    bucket_targets: Vec<BucketTarget>,
}

#[derive(Debug, Clone, Default)]
pub struct BucketTarget {
    bucket: String,
    target: String,
}
