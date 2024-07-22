use base62::encode;
use uuid::Uuid;

pub fn get_uuid() -> String {
  let full_uuid = Uuid::new_v4();
  let bytes = full_uuid.as_bytes();

  // Convert the byte array to a u128
  let mut array = [0u8; 16];
  array.copy_from_slice(bytes);
  let num = u128::from_be_bytes(array);

  encode(num)
}
