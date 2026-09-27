# Selected operation validation seam

This is an unfrozen private-test seam in the shared
[candidate reader](../../crates/photara-store/tests/integrated_wire_candidate/wire.rs).
It does not change production readers, permanent fields or storage qualification.

`OperationSelection` carries an exact expected commit, the exact current loose
control references and the original packed hold reference. A compiled finite
route must derive that input from parsed original admission and phase bytes; it
must not copy an unvalidated current commit into the expected value. The route
owns exact O/P/F identity, supported codec/stage dispatch, original reserve,
consumption, remaining reserve and nonempty hold validation.

`verify_selected_operation` first authenticates the original package through the
existing full verifier. It independently requires the supplied packed hold to
equal that original envelope's hold. The current commit must equal the derived
selection, and the exact loose control set must include the actual selected
envelope, ledger and current hold, with no duplicates. Actual HEAD, typed root
closures, physical locators, allocation ownership, accounting and global union
continue through the shared driver.

The generic accounting component still reads the original packed empty hold;
it does not gain support for arbitrary nonempty holds through this seam. The
route must prove current operation controls separately. For a prepublication
stage, a historical prefix view proves only that original closure. Every live
suffix must separately match the original deterministic recipe and phase
corridor. A clipped prefix is not evidence of current physical accounting.

`recovery_operation` uses the same selected bootstrap and exact control-set
checks with the existing independent recovery closure path. Its route caller
must first validate selected operation metadata. It can inspect the supplied
recovery closure when unrelated active allocations are absent; it does not
verify those absent bytes, establish full global accounting or authorize writes.

Existing settled, origin and prepared-successor entry points retain their
default behavior. The [source-bound regression evidence](verification/ps2-selected-operation-seam.json)
records this narrow checkpoint. The selected phase route, its original-prefix
Core replay, adversarial cuts and finite bounds require their own complete
evidence package. Neither this seam nor its default regressions close those
route-level obligations.
