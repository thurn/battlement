fn main() {
  let count = asset_registry_consumer::registration_count();
  let hash = asset_registry_consumer::registration_address_hash();
  assert_eq!(count, 2);
  assert_ne!(hash, 0);
  println!("registrations={count} address_hash={hash:08x}");
}
