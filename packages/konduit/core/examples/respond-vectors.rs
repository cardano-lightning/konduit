use konduit_core::{Indexes, SigningKey, Squash, SquashBody, Tag};

fn main() {
    let key = SigningKey::from([0; 32]);
    let tag = Tag::from(b"konduit-core-test-vector-v1".as_slice());
    let body = SquashBody::new(0, 0, Indexes::new(vec![]).unwrap()).unwrap();
    let squash = Squash::make(&key, &tag, body);
    println!(
        "use aiken/crypto.{{VerificationKey}}\n\npub const key: VerificationKey =\n  #\"{}\"\n\npub const tag: ByteArray =\n  #\"{}\"\n\npub const squash =\n  (\n    (0, 0, []),\n    #\"{}\",\n  )\n\npub type RespondVector {{\n  RespondVector {{\n    name: ByteArray,\n    upper_bound: Int,\n    expected_return: VerificationKey,\n  }}\n}}\n\npub const vectors: List<RespondVector> = [\n  RespondVector {{ name: \"respond_empty_cheques\", upper_bound: 1000, expected_return: #\"{}\" }},\n]",
        hex::encode(key.verifying_key().to_bytes()),
        hex::encode(tag.as_ref()),
        hex::encode(squash.signature().to_bytes()),
        hex::encode(key.verifying_key().to_bytes())
    );
}
