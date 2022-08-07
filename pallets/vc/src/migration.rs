use super::*;

pub fn migrate<T: Config>() -> frame_support::weights::Weight {
    frame_support::debug::RuntimeLogger::init();
    // Storage migrations should use storage versions for safety.
    match PalletVersion::get() {
        VCPalletVersion::V1_0_0 => {
            for (vc_id, (vc, _)) in VCs::<T>::iter() {
              set_approved_issuers::<T>(vc_id, &vc);
            }
            // Update storage version.
            PalletVersion::put(VCPalletVersion::V2_0_0);

            let count = VCApproverList::iter().count();
            // Return the weight consumed by the migration.
            T::DbWeight::get().reads_writes(count as Weight + 1, count as Weight + 1)
        }
        _ => {
            frame_support::debug::info!(" >>> Unused migration!");
            0
        }
    }
}

fn set_approved_issuers<T: Config>(vc_id: VCid, vc: &VC<T::Hash>) {
  let mut vc_approver_list = VCApproverList::get(vc_id);
  let signatures = vc.signatures.clone();
  // Check approved signatures
  for i in 0..signatures.len() {
      let sign = &signatures[i];
      for issuer in vc.issuers.iter() {
          let (issuer_details, _) = did::Module::<T>::get_did_details(*issuer).unwrap();
          if sign.verify(vc.hash.as_ref(), &issuer_details.public_key) {
              if !vc_approver_list.contains(&issuer_details.identifier) {
                vc_approver_list.push(issuer_details.identifier);
              }
          }
      }
  }
  VCApproverList::insert(vc_id, vc_approver_list);
}