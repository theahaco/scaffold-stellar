# Registry IssueOps — operator runbook

This repo runs a GitHub IssueOps pipeline for the on-chain Stellar Registry. It mirrors the pattern used by [SCF Public Goods Maintenance](https://github.com/SCF-Public-Goods-Maintenance/scf-public-goods-maintenance.github.io): structured issue forms drive a label-based state machine, a pilot team votes via reactions, and an accepted intake fires a GitHub-environment-gated workflow that signs the on-chain transaction with a CI key.

## What lives where

| File | What it does |
|---|---|
| `.github/ISSUE_TEMPLATE/registry-{publish,register,deploy,admin}.yml` | Four issue forms — one per registry method family. The form sets the `registry-intake` parent label and a `registry-intake:<kind>` subtype. |
| `.github/validator/config.yml` + `validate-*.js` | Field-level validation (Stellar strkeys, semver, wasm hashes, contract names, https URLs). |
| `.github/registry-quorum.yml` | Per-kind quorum policy: pilot team slug, `min_voters`, `require_unanimous`. Edit via PR. |
| `.github/workflows/validate-registry-intake.yml` | Runs on issue open/edit/reopen. Parses + validates the form; on success labels `:in-review`, on failure labels `issueops:validation-error` and posts a friendly comment. |
| `.github/workflows/registry-quorum-check.yml` | Runs on `.quorum-check` comments. Tallies pilot 👍/👎, applies the policy, dispatches the on-chain submit workflow on accept (or closes the issue on reject). |
| `.github/workflows/registry-onchain-submit.yml` | `workflow_dispatch` only. Re-fetches and re-validates the issue body, parses to CLI args, runs `stellar-registry-cli` from a protected GitHub Environment with the CI signer secret. |
| `.github/scripts/parse_intake_to_args.py` | Single source of truth for the kind → CLI subcommand → flags mapping. Edit via PR. |
| `.github/scripts/render_validation_failure.py` | Pretty-prints validator errors into an issue comment. |

## Architecture at a glance

```
issue-form-submit ─▶ validate-registry-intake.yml ─▶ :in-review
       │                          │
       │                          └─ on fail ▶ issueops:validation-error + comment
       ▼
pilots react 👍/👎
.quorum-check comment ─▶ registry-quorum-check.yml
       │                          │
       │              ┌──── reject path ────┐
       │              ▼                     │
       │   :rejected + close+lock           │
       │                                    │
       └────── accept path ─────────────────┘
                       │
                       ▼
       :accepted + dispatch
                       │
                       ▼
   registry-onchain-submit.yml (workflow_dispatch)
                       │
              ─────────┴───────── GitHub Environment gate (required reviewers)
                       │
                       ▼
            stellar-registry-cli <method> --source ci ...
                       │
                       ▼
         :submitted + tx hash comment ─OR─ :submission-failed
```

The CI key is set as the registry contract's **manager** (see `contracts/registry/src/lib.rs:80-82`). When a manager is set, `publish`, `register_contract`, `deploy`, `update_*`, `rename_contract`, and `flag_contract` all accept manager auth in lieu of the original author/admin/owner. That is the contract-side hook that lets one CI key cover all of v1.

## Bootstrapping (one-time per network)

Do these in order. All steps that touch the registry contract are signed locally by the registry **admin**, not by CI.

### 1. Create the pilots team

In the GitHub org that owns this repo, create a team named `registry-pilots`. Add at least 3 members (mirroring `defaults.min_voters` in `.github/registry-quorum.yml`). Members of this team are the only accounts whose 👍/👎 reactions count toward quorum, and only members may run `.quorum-check`.

### 2. Provision the org-read PAT

The `registry-quorum-check.yml` workflow needs to enumerate `registry-pilots` members. The default `GITHUB_TOKEN` cannot read org teams, so create a fine-grained PAT:

- Resource owner: the org
- Repository access: this repo only
- Organization permissions: **Members → Read**

Store as repo secret `READ_ORG_MEMBERS_PAT`. Mirrors pg-atlas's setup.

### 3. Generate the CI key

Offline (developer workstation), per network:

```sh
stellar keys generate ci-publisher  # testnet auto-funds; for mainnet, fund out-of-band
stellar keys public-key ci-publisher  # → CI_PUBKEY
stellar keys show ci-publisher        # → CI_SECRET (handle with care)
```

Treat the mainnet secret as you would any production credential — keep the offline copy in a password manager / hardware wallet only, and ensure no one can recover it from `STELLAR_SECRET_KEY` once it lands in GitHub.

### 4. Set the CI key as the registry manager

The registry **admin** (whoever holds the admin keypair from registry deployment) runs once per network:

```sh
stellar contract invoke \
  --id "$STELLAR_REGISTRY_CONTRACT_ID" \
  --source admin \
  --network <testnet|mainnet> \
  -- set_manager --new_manager "$CI_PUBKEY"
```

Verify:

```sh
stellar contract invoke --id "$STELLAR_REGISTRY_CONTRACT_ID" --network <…> -- manager
# → "$CI_PUBKEY"
```

### 5. Create GitHub Environments

In repo settings → Environments, create two:

- `registry-testnet`
- `registry-mainnet`

For each, attach **required reviewers** (recommend the same set as `registry-pilots` for testnet; a stricter ops subset for mainnet). The env reviewer prompt is the second gate after the pilot quorum vote.

For each environment set:

| Kind | Name | Value |
|---|---|---|
| Secret | `STELLAR_SECRET_KEY` | The CI key's secret seed (`SC…`). |
| Secret | `STELLAR_NETWORK_PASSPHRASE` | `Test SDF Network ; September 2015` (testnet) or `Public Global Stellar Network ; September 2015` (mainnet). |
| Variable | `STELLAR_RPC_URL` | The RPC endpoint for that network. |
| Variable | `STELLAR_REGISTRY_CONTRACT_ID` | The registry contract's `C…` address on that network. |

## Day-to-day usage

### Submitter workflow

1. Click **New issue** in this repo. Pick one of the four "Registry: …" forms.
2. Fill in the fields. The form's `validations: required: true` blocks empty submission for must-have fields; the workflow re-validates kind-specific cross-field rules (e.g. `update_contract_owner` requires `new_owner`).
3. On submit, the validator workflow runs. If it labels you `issueops:validation-error`, fix the fields and edit the issue — the workflow re-runs on edit.
4. Once labeled `registry-intake:in-review`, wait for pilot votes.

### Pilot workflow

1. Read the issue, including the **Justification** section.
2. React with 👍 to approve or 👎 to reject. (One reaction of each kind per pilot — the API enforces this.)
3. When you believe the quorum threshold has been reached, comment exactly `.quorum-check`. Only `registry-pilots` members can trigger this; the action checks via the org PAT.
4. The workflow tallies according to `.github/registry-quorum.yml`:
   - **Default formula:** `accepted iff ups ≥ (min_voters + 2 × downs)`. So 3 ups beats 0 downs; 5 ups beats 1 down.
   - **`require_unanimous: true`** (admin kind): any 👎 blocks acceptance regardless of 👍 count.
5. On accept, the on-chain submit workflow is dispatched. It will pause at the env gate; an environment reviewer must click **Approve and deploy** before the CI key signs.
6. On reject, the issue is closed and locked.

### Where to find the result

A successful submission posts a comment on the issue with the tx hash, links to the workflow run, and applies the `registry-intake:submitted` label. A failed submission applies `registry-intake:submission-failed` and links to the run logs; a maintainer can re-trigger via the workflow_dispatch UI after fixing the underlying problem.

## Key rotation

```
Generate new CI key offline
  ↓
admin: set_manager(<NEW_PUBKEY>)
  ↓
Update each Environment's STELLAR_SECRET_KEY secret to the new SC…
  ↓
(Optional, hardens the gap) admin: remove_manager + set_manager(<NEW_PUBKEY>)
  ↓
Old key now has no on-chain authority; can be retired
```

There is intentionally no "rotate from CI" path — rotation is an admin-only ceremony.

## Out of scope for v1 (offline-only, admin-signed)

The following operations require **registry admin** auth, not the manager / CI key. They are not exposed as IssueOps forms. To perform them, sign locally with the registry admin keypair.

| Operation | Why offline |
|---|---|
| `set_admin` | Changing who can change the manager. Highest blast radius. |
| `set_manager` | Including initial bootstrap and key rotation. Admin-signed by design. |
| `remove_manager` | Same. |
| `upgrade` (registry self-upgrade) | Replaces the registry contract itself. Out of scope for IssueOps. |

## Future migration: Tansu factory pattern

When per-org contracts land:

1. Deploy the org contract `O` with multi-sig logic via Soroban `__check_auth`.
2. Admin runs `set_manager(O)`.
3. Update `registry-onchain-submit.yml`: instead of submitting a tx fully signed by the CI key, submit a tx whose auth entry references `O`. `O.__check_auth` reads quorum signatures from its own bag of signers.
4. The CI key is retained as one of N quorum signers, or retired entirely if all signers are humans / hardware wallets.

The IssueOps pipeline (forms, validation, pilot vote, env gate) stays unchanged. It now governs **one** of N signatures rather than the sole authority. This is the explicit reason to invest in IssueOps now: the durable shape of human-mediated approvals doesn't change when the contract-side authority model becomes more sophisticated.
