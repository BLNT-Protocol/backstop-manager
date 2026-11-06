# Dependencies

| File | Source | SHA-256 |
| --- | --- | --- |
| `comet.wasm` | Official Comet V1.1 `v1.1.0_contracts_pkg1.0.0_cli25.1.0` release (same as `blnt-contracts-v3/comet.wasm`) | `d735c3395f59510172cf5cf838823a1389b97cfec6bf24e580e9bd77d2b3e687` |
| `backstop.wasm` | `blnt-contracts-v3` @ `5d055ca`, `stellar contract build --package backstop --locked` | `d07a0a49cb9004a7601d5b8e156bb111277a248b3cec23f6e04c8722baa64b85` |

## Test only

These are used only to deploy a BLNT v3 environment in tests. They are built
from `blnt-contracts-v3` @ `5d055ca` with `stellar contract build --package <pkg> --locked`.

| File | SHA-256 |
| --- | --- |
| `test/emitter.wasm` | `8c9133ed2c28685f16e4edc4d991505b6b07d1142f6698facf3239d088722e3d` |
| `test/pool_factory.wasm` | `0dacf658f3c202bfcf90ed33ec7196727d23d8c50149f13a88ec2022bf53a0ad` |
| `test/pool.wasm` | `d448943bf5876e55a7f4fb58327893d1f20e55c90580fc60fdd49da9ebbf51cf` |

These builds don't include the `source_repo` meta that the v3 `make build` adds,
so their hashes won't match the published v3 release artifacts. Rebuild them
from a tagged v3 release before deploying to production.
