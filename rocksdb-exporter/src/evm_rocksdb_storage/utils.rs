use fmerk::tree;

pub(crate) fn decode_kv(kv_pair: &(Box<[u8]>, Box<[u8]>)) -> (Vec<u8>, Vec<u8>) {
    let kv = tree::Tree::decode(kv_pair.0.to_vec(), &kv_pair.1);
    (kv.key().to_vec(), kv.value().to_vec())
}
