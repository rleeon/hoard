Fixtures consumed by `detection_integration.rs`.

The detection pipeline is end-to-end-tested against tempdir-backed Steam
installs, Proton prefixes and Paradox-style game roots that the tests build
on the fly via the helpers at the top of `detection_integration.rs`. Nothing
in this directory ships in the binary; checked in only so the layout stays
visible to anyone browsing the crate (and so the test harness has a stable
place to drop generated material if it ever needs to).
