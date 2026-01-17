<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

# Feature Flags, Experiments & Analytics Test Plan

Manual test plan for exercising all Feature Flag, Experiment, and PostHog-style Analytics endpoints via `curl`.

> **Note:** API routes use `/api/` prefix. All endpoints require authentication unless otherwise noted.

## Prerequisites

### Environment Setup

```bash
# Server URL
export LOOM_SERVER=https://loom.ghuntley.com

# Login via loom CLI
loom --server-url $LOOM_SERVER login

# Extract token for curl (stored in credentials file)
export LOOM_TOKEN=$(jq -r '.https___loom_ghuntley_com.ApiKey.key // .https___loom_ghuntley_com.OAuth.access' ~/.config/loom/credentials.json)

# Get your user ID and org ID
export USER_ID="<your-user-uuid>"
export ORG_ID="<your-org-uuid>"

# Test directory
mkdir -p /tmp/loom-flags-tests
cd /tmp/loom-flags-tests
```

---

## Part 1: Feature Flags & Experiments

### 1. Environment Management

#### 1.1 List Environments (Empty)

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags/environments" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"environments": []}
```

#### 1.2 Create Environment

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/environments" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "production",
    "color": "#22c55e"
  }' | jq .
# Expected: 201 Created with environment details
```

**Save environment ID:**
```bash
export ENV_ID=$(curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/environments" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "staging", "color": "#f59e0b"}' | jq -r '.id')
echo "Created environment: $ENV_ID"
```

#### 1.3 Get Environment

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags/environments/$ENV_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with environment details
```

#### 1.4 Update Environment

```bash
curl -s -X PATCH "$LOOM_SERVER/api/orgs/$ORG_ID/flags/environments/$ENV_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "staging-updated", "color": "#3b82f6"}' | jq .
# Expected: 200 OK with updated environment
```

#### 1.5 List Environments (After Create)

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags/environments" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with environments array
```

#### 1.6 Delete Environment

```bash
curl -s -X DELETE "$LOOM_SERVER/api/orgs/$ORG_ID/flags/environments/$ENV_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"message": "..."}
```

---

### 2. SDK Key Management

#### 2.1 List SDK Keys (Empty)

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags/environments/$ENV_ID/sdk-keys" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"sdk_keys": []}
```

#### 2.2 Create Client-Side SDK Key

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/environments/$ENV_ID/sdk-keys" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "environment_id": "'"$ENV_ID"'",
    "key_type": "client_side",
    "name": "Web App Key"
  }' | jq .
# Expected: 201 Created with {id, key, environment_id, key_type, name, created_at}
# IMPORTANT: Save the 'key' value - only shown once!
```

**Save SDK key:**
```bash
export SDK_KEY=$(curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/environments/$ENV_ID/sdk-keys" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"environment_id": "'"$ENV_ID"'", "key_type": "server_side", "name": "Server Key"}' | jq -r '.key')
export SDK_KEY_ID=$(curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags/environments/$ENV_ID/sdk-keys" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq -r '.sdk_keys[0].id')
echo "SDK Key: $SDK_KEY"
```

#### 2.3 Create Server-Side SDK Key

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/environments/$ENV_ID/sdk-keys" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "environment_id": "'"$ENV_ID"'",
    "key_type": "server_side",
    "name": "Backend Service Key"
  }' | jq .
# Expected: 201 Created
```

#### 2.4 List SDK Keys (After Create)

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags/environments/$ENV_ID/sdk-keys" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with sdk_keys array (key field redacted)
```

#### 2.5 Revoke SDK Key

```bash
curl -s -X DELETE "$LOOM_SERVER/api/orgs/$ORG_ID/flags/sdk-keys/$SDK_KEY_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"message": "..."}
```

---

### 3. Feature Flag Management

#### 3.1 List Flags (Empty)

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"flags": []}
```

#### 3.2 Create Boolean Flag

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "key": "feature.new_checkout",
    "name": "New Checkout Flow",
    "description": "Enable the new checkout experience",
    "tags": ["checkout", "experiment"],
    "variants": [
      {"name": "off", "value": {"type": "Boolean", "value": false}, "weight": 50},
      {"name": "on", "value": {"type": "Boolean", "value": true}, "weight": 50}
    ],
    "default_variant": "off"
  }' | jq .
# Expected: 201 Created with flag details
```

**Save flag ID:**
```bash
export FLAG_ID=$(curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"key": "feature.test_flag", "name": "Test Flag", "variants": [{"name": "off", "value": {"type": "Boolean", "value": false}, "weight": 100}], "default_variant": "off"}' \
  | jq -r '.id')
echo "Created flag: $FLAG_ID"
```

#### 3.3 Create String Variant Flag

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "key": "ui.theme",
    "name": "UI Theme",
    "description": "Control the UI theme variant",
    "variants": [
      {"name": "light", "value": {"type": "String", "value": "light"}, "weight": 33},
      {"name": "dark", "value": {"type": "String", "value": "dark"}, "weight": 33},
      {"name": "auto", "value": {"type": "String", "value": "auto"}, "weight": 34}
    ],
    "default_variant": "light"
  }' | jq .
# Expected: 201 Created
```

#### 3.4 Create JSON Config Flag

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "key": "config.rate_limits",
    "name": "Rate Limits Config",
    "variants": [
      {"name": "default", "value": {"type": "Json", "value": {"requests_per_minute": 60, "burst": 10}}, "weight": 100}
    ],
    "default_variant": "default"
  }' | jq .
# Expected: 201 Created
```

#### 3.5 Create Flag with Prerequisites

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "key": "feature.advanced_checkout",
    "name": "Advanced Checkout",
    "variants": [
      {"name": "off", "value": {"type": "Boolean", "value": false}, "weight": 100}
    ],
    "default_variant": "off",
    "prerequisites": [
      {"flag_key": "feature.new_checkout", "required_variant": "on"}
    ]
  }' | jq .
# Expected: 201 Created
```

#### 3.6 Invalid Flag Key (Should Fail)

```bash
# Key with spaces
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"key": "invalid key", "name": "Test", "variants": [{"name": "off", "value": {"type": "Boolean", "value": false}, "weight": 100}], "default_variant": "off"}'
# Expected: 400 Bad Request

# Key too short
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"key": "ab", "name": "Test", "variants": [{"name": "off", "value": {"type": "Boolean", "value": false}, "weight": 100}], "default_variant": "off"}'
# Expected: 400 Bad Request
```

#### 3.7 Get Flag

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags/$FLAG_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with full flag details
```

#### 3.8 Update Flag

```bash
curl -s -X PATCH "$LOOM_SERVER/api/orgs/$ORG_ID/flags/$FLAG_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Flag Updated",
    "description": "Updated description",
    "tags": ["updated", "test"]
  }' | jq .
# Expected: 200 OK with updated flag
```

#### 3.9 List Flags (After Create)

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with flags array
```

#### 3.10 List Flags Including Archived

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags?include_archived=true" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with all flags including archived
```

#### 3.11 Archive Flag

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/$FLAG_ID/archive" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"message": "..."}
```

#### 3.12 Restore Flag

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/$FLAG_ID/restore" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"message": "..."}
```

---

### 4. Flag Configuration (Per-Environment)

#### 4.1 List Flag Configs

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags/$FLAG_ID/configs" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"configs": [...]}
```

#### 4.2 Get Flag Config for Environment

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags/$FLAG_ID/configs/$ENV_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with config details
```

#### 4.3 Update Flag Config (Enable)

```bash
curl -s -X PATCH "$LOOM_SERVER/api/orgs/$ORG_ID/flags/$FLAG_ID/configs/$ENV_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}' | jq .
# Expected: 200 OK with updated config
```

#### 4.4 Update Flag Config (Assign Strategy)

```bash
curl -s -X PATCH "$LOOM_SERVER/api/orgs/$ORG_ID/flags/$FLAG_ID/configs/$ENV_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"strategy_id": "'"$STRATEGY_ID"'"}' | jq .
# Expected: 200 OK with updated config
```

---

### 5. Strategy Management

#### 5.1 List Strategies (Empty)

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags/strategies" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"strategies": []}
```

#### 5.2 Create Percentage Rollout Strategy

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/strategies" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "50% Rollout",
    "description": "Roll out to 50% of users",
    "percentage": 50,
    "percentage_key": "user_id"
  }' | jq .
# Expected: 201 Created
```

**Save strategy ID:**
```bash
export STRATEGY_ID=$(curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/strategies" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Test Strategy", "percentage": 100}' | jq -r '.id')
echo "Created strategy: $STRATEGY_ID"
```

#### 5.3 Create Attribute-Based Strategy

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/strategies" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Pro Users Only",
    "description": "Target users with pro plan",
    "conditions": [
      {
        "type": "Attribute",
        "attribute": "plan",
        "operator": "equals",
        "value": "pro"
      }
    ]
  }' | jq .
# Expected: 201 Created
```

#### 5.4 Create Geographic Strategy

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/strategies" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "US Only",
    "description": "Target US users",
    "conditions": [
      {
        "type": "Geographic",
        "field": "country",
        "values": ["US", "CA"]
      }
    ]
  }' | jq .
# Expected: 201 Created
```

#### 5.5 Create Scheduled Rollout Strategy

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/strategies" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Gradual Rollout",
    "description": "10% to 100% over time",
    "schedule": {
      "steps": [
        {"percentage": 10, "start_at": "2026-01-20T00:00:00Z"},
        {"percentage": 50, "start_at": "2026-01-25T00:00:00Z"},
        {"percentage": 100, "start_at": "2026-02-01T00:00:00Z"}
      ]
    }
  }' | jq .
# Expected: 201 Created
```

#### 5.6 Get Strategy

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags/strategies/$STRATEGY_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK
```

#### 5.7 Update Strategy

```bash
curl -s -X PATCH "$LOOM_SERVER/api/orgs/$ORG_ID/flags/strategies/$STRATEGY_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Updated Strategy", "percentage": 75}' | jq .
# Expected: 200 OK
```

#### 5.8 Delete Strategy

```bash
curl -s -X DELETE "$LOOM_SERVER/api/orgs/$ORG_ID/flags/strategies/$STRATEGY_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"message": "..."}
```

---

### 6. Kill Switch Management

#### 6.1 List Kill Switches (Empty)

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags/kill-switches" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"kill_switches": []}
```

#### 6.2 Create Kill Switch

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/kill-switches" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "key": "checkout_emergency",
    "name": "Checkout Emergency Kill",
    "description": "Emergency disable for checkout features",
    "linked_flag_keys": ["feature.new_checkout", "feature.advanced_checkout"]
  }' | jq .
# Expected: 201 Created
```

**Save kill switch ID:**
```bash
export KILL_SWITCH_ID=$(curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/kill-switches" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"key": "test_kill_switch", "name": "Test Kill Switch"}' | jq -r '.id')
echo "Created kill switch: $KILL_SWITCH_ID"
```

#### 6.3 Get Kill Switch

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags/kill-switches/$KILL_SWITCH_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK
```

#### 6.4 Update Kill Switch

```bash
curl -s -X PATCH "$LOOM_SERVER/api/orgs/$ORG_ID/flags/kill-switches/$KILL_SWITCH_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Updated Kill Switch",
    "linked_flag_keys": ["feature.new_checkout"]
  }' | jq .
# Expected: 200 OK
```

#### 6.5 Activate Kill Switch

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/kill-switches/$KILL_SWITCH_ID/activate" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"reason": "High error rate detected in checkout flow"}' | jq .
# Expected: 200 OK with is_active: true
```

#### 6.6 Deactivate Kill Switch

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/kill-switches/$KILL_SWITCH_ID/deactivate" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with is_active: false
```

#### 6.7 List Kill Switches (After Create)

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags/kill-switches" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with kill_switches array
```

#### 6.8 Delete Kill Switch

```bash
curl -s -X DELETE "$LOOM_SERVER/api/orgs/$ORG_ID/flags/kill-switches/$KILL_SWITCH_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"message": "..."}
```

---

### 7. Flag Evaluation

#### 7.1 Evaluate Single Flag

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/feature.new_checkout/evaluate" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "context": {
      "user_id": "user-123",
      "org_id": "'"$ORG_ID"'",
      "environment": "production",
      "attributes": {
        "plan": "pro",
        "signup_date": "2025-01-01"
      }
    }
  }' | jq .
# Expected: 200 OK with {flag_key, variant, value, reason}
```

#### 7.2 Evaluate All Flags

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/evaluate" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "context": {
      "user_id": "user-123",
      "environment": "production",
      "attributes": {"plan": "pro"}
    }
  }' | jq .
# Expected: 200 OK with {results: [...], evaluated_at}
```

#### 7.3 Evaluate with Geographic Context

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/evaluate" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "context": {
      "user_id": "user-456",
      "environment": "production",
      "geo": {
        "country": "US",
        "region": "CA",
        "city": "San Francisco"
      }
    }
  }' | jq .
# Expected: 200 OK
```

#### 7.4 Evaluate Non-Existent Flag

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/flags/nonexistent.flag/evaluate" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"context": {"environment": "production"}}'
# Expected: 404 Not Found
```

---

### 8. Flag Analytics

#### 8.1 Get Flag Stats

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags/feature.new_checkout/stats" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {flag_key, last_evaluated_at, evaluation_count_24h, evaluation_count_7d, evaluation_count_30d}
```

#### 8.2 List Stale Flags

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/flags/stale" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {stale_flags: [...], stale_threshold_days}
```

---

### 9. Real-Time Flag Stream (SSE)

#### 9.1 Connect to Flag Stream

```bash
# This will stream events - press Ctrl+C to stop
curl -N "$LOOM_SERVER/api/flags/stream?environment=production" \
  -H "Authorization: Bearer $SDK_KEY"
# Expected: SSE stream with init event containing current flag states
```

#### 9.2 Stream Stats (Admin)

```bash
curl -s "$LOOM_SERVER/api/flags/stream/stats" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with streaming stats (requires system_admin)
```

---

### 10. Platform-Level Admin Flags (System Admin Only)

> **Note:** These endpoints require `is_system_admin = true` on the user account.

#### 10.1 List Platform Flags

```bash
curl -s "$LOOM_SERVER/api/admin/flags" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with platform-wide flags (or 403 if not admin)
```

#### 10.2 Create Platform Flag

```bash
curl -s -X POST "$LOOM_SERVER/api/admin/flags" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "key": "platform.maintenance_mode",
    "name": "Maintenance Mode",
    "variants": [
      {"name": "off", "value": {"type": "Boolean", "value": false}, "weight": 100}
    ],
    "default_variant": "off"
  }' | jq .
# Expected: 201 Created (or 403 if not admin)
```

#### 10.3 List Platform Kill Switches

```bash
curl -s "$LOOM_SERVER/api/admin/flags/kill-switches" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK (or 403 if not admin)
```

#### 10.4 Activate Platform Kill Switch

```bash
curl -s -X POST "$LOOM_SERVER/api/admin/flags/kill-switches/$KEY/activate" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"reason": "Platform maintenance"}' | jq .
# Expected: 200 OK (or 403 if not admin)
```

---

## Part 2: PostHog-Style Analytics

### 11. Analytics API Key Management

#### 11.1 List Analytics API Keys (Empty)

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/analytics/api-keys" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"api_keys": []}
```

#### 11.2 Create Write-Only API Key

```bash
curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/analytics/api-keys" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Web App Tracking",
    "key_type": "write"
  }' | jq .
# Expected: 201 Created with {id, key, name, key_type, created_at}
# Key prefix: loom_analytics_write_
# IMPORTANT: Save the 'key' value - only shown once!
```

**Save analytics API key:**
```bash
export ANALYTICS_WRITE_KEY=$(curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/analytics/api-keys" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Test Write Key", "key_type": "write"}' | jq -r '.key')
echo "Write Key: $ANALYTICS_WRITE_KEY"
```

#### 11.3 Create ReadWrite API Key

```bash
export ANALYTICS_RW_KEY=$(curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_ID/analytics/api-keys" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Server Analytics Key", "key_type": "read_write"}' | jq -r '.key')
echo "ReadWrite Key: $ANALYTICS_RW_KEY"
# Key prefix: loom_analytics_rw_
```

#### 11.4 List Analytics API Keys (After Create)

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/analytics/api-keys" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with api_keys array (key field NOT shown)
```

#### 11.5 Revoke Analytics API Key

```bash
export ANALYTICS_KEY_ID=$(curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/analytics/api-keys" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq -r '.api_keys[0].id')
curl -s -X DELETE "$LOOM_SERVER/api/orgs/$ORG_ID/analytics/api-keys/$ANALYTICS_KEY_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"message": "API key revoked successfully"}
```

#### 11.6 Revoke Already Revoked Key (Should Fail)

```bash
curl -s -X DELETE "$LOOM_SERVER/api/orgs/$ORG_ID/analytics/api-keys/$ANALYTICS_KEY_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN"
# Expected: 400 Bad Request with "already_revoked" error
```

---

### 12. Event Capture

#### 12.1 Capture Single Event

```bash
curl -s -X POST "$LOOM_SERVER/api/analytics/capture" \
  -H "Authorization: Bearer $ANALYTICS_WRITE_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "distinct_id": "user-123",
    "event": "button_clicked",
    "properties": {
      "button_name": "checkout",
      "page": "/cart",
      "value": 99.99
    }
  }' | jq .
# Expected: 200 OK with {status: "ok", event_id: "..."}
```

#### 12.2 Capture Event with Timestamp

```bash
curl -s -X POST "$LOOM_SERVER/api/analytics/capture" \
  -H "Authorization: Bearer $ANALYTICS_WRITE_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "distinct_id": "user-123",
    "event": "purchase_completed",
    "properties": {"amount": 149.99, "currency": "USD"},
    "timestamp": "2026-01-18T10:30:00Z"
  }' | jq .
# Expected: 200 OK
```

#### 12.3 Capture Batch of Events

```bash
curl -s -X POST "$LOOM_SERVER/api/analytics/batch" \
  -H "Authorization: Bearer $ANALYTICS_WRITE_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "batch": [
      {"distinct_id": "user-123", "event": "page_viewed", "properties": {"page": "/home"}},
      {"distinct_id": "user-123", "event": "page_viewed", "properties": {"page": "/pricing"}},
      {"distinct_id": "user-456", "event": "signup_started", "properties": {"source": "google"}}
    ]
  }' | jq .
# Expected: 200 OK with {status: "ok", count: 3}
```

#### 12.4 Capture with Special Event Names

```bash
# $pageview (system event)
curl -s -X POST "$LOOM_SERVER/api/analytics/capture" \
  -H "Authorization: Bearer $ANALYTICS_WRITE_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "distinct_id": "user-123",
    "event": "$pageview",
    "properties": {"$current_url": "https://example.com/pricing", "$pathname": "/pricing"}
  }' | jq .
# Expected: 200 OK
```

#### 12.5 Invalid Event Name (Should Fail)

```bash
curl -s -X POST "$LOOM_SERVER/api/analytics/capture" \
  -H "Authorization: Bearer $ANALYTICS_WRITE_KEY" \
  -H "Content-Type: application/json" \
  -d '{"distinct_id": "user-123", "event": "invalid event with spaces"}'
# Expected: 400 Bad Request with "invalid_event_name" error
```

#### 12.6 Empty Distinct ID (Should Fail)

```bash
curl -s -X POST "$LOOM_SERVER/api/analytics/capture" \
  -H "Authorization: Bearer $ANALYTICS_WRITE_KEY" \
  -H "Content-Type: application/json" \
  -d '{"distinct_id": "", "event": "test_event"}'
# Expected: 400 Bad Request with "invalid_distinct_id" error
```

#### 12.7 Empty Batch (Should Fail)

```bash
curl -s -X POST "$LOOM_SERVER/api/analytics/batch" \
  -H "Authorization: Bearer $ANALYTICS_WRITE_KEY" \
  -H "Content-Type: application/json" \
  -d '{"batch": []}'
# Expected: 400 Bad Request with "empty_batch" error
```

#### 12.8 Batch Too Large (Should Fail)

```bash
# Generate 101 events (over the 100 limit)
EVENTS=$(for i in $(seq 1 101); do echo '{"distinct_id": "user", "event": "test"}'; done | jq -s '.')
curl -s -X POST "$LOOM_SERVER/api/analytics/batch" \
  -H "Authorization: Bearer $ANALYTICS_WRITE_KEY" \
  -H "Content-Type: application/json" \
  -d "{\"batch\": $EVENTS}"
# Expected: 400 Bad Request with "batch_too_large" error
```

#### 12.9 Capture Without Auth (Should Fail)

```bash
curl -s -X POST "$LOOM_SERVER/api/analytics/capture" \
  -H "Content-Type: application/json" \
  -d '{"distinct_id": "user-123", "event": "test"}'
# Expected: 401 Unauthorized
```

#### 12.10 Capture with Revoked Key (Should Fail)

```bash
curl -s -X POST "$LOOM_SERVER/api/analytics/capture" \
  -H "Authorization: Bearer loom_analytics_write_revoked_key_12345" \
  -H "Content-Type: application/json" \
  -d '{"distinct_id": "user-123", "event": "test"}'
# Expected: 401 Unauthorized
```

---

### 13. Identity Operations

#### 13.1 Identify User

```bash
curl -s -X POST "$LOOM_SERVER/api/analytics/identify" \
  -H "Authorization: Bearer $ANALYTICS_WRITE_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "distinct_id": "550e8400-e29b-41d4-a716-446655440000",
    "user_id": "user@example.com",
    "properties": {
      "email": "user@example.com",
      "plan": "pro",
      "company": "Acme Corp"
    }
  }' | jq .
# Expected: 200 OK with {status: "ok", person_id: "..."}
```

#### 13.2 Alias Two Identities

```bash
curl -s -X POST "$LOOM_SERVER/api/analytics/alias" \
  -H "Authorization: Bearer $ANALYTICS_WRITE_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "distinct_id": "user@example.com",
    "alias": "user-internal-id-12345"
  }' | jq .
# Expected: 200 OK with {status: "ok", person_id: "..."}
```

#### 13.3 Set Person Properties

```bash
curl -s -X POST "$LOOM_SERVER/api/analytics/set" \
  -H "Authorization: Bearer $ANALYTICS_WRITE_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "distinct_id": "user@example.com",
    "properties": {
      "last_login": "2026-01-18T10:30:00Z",
      "feature_usage": {"dashboard": 42, "reports": 15}
    }
  }' | jq .
# Expected: 200 OK with {status: "ok", person_id: "..."}
```

#### 13.4 Set Properties with set_once

```bash
curl -s -X POST "$LOOM_SERVER/api/analytics/set" \
  -H "Authorization: Bearer $ANALYTICS_WRITE_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "distinct_id": "user@example.com",
    "properties": {
      "first_seen": "2026-01-18T10:30:00Z",
      "original_source": "google"
    },
    "set_once": true
  }' | jq .
# Expected: 200 OK (only sets properties that don't exist)
```

---

### 14. Query Endpoints (Requires ReadWrite Key)

#### 14.1 List Persons

```bash
curl -s "$LOOM_SERVER/api/analytics/persons?limit=10&offset=0" \
  -H "Authorization: Bearer $ANALYTICS_RW_KEY" | jq .
# Expected: 200 OK with {persons: [...], total, limit, offset}
```

#### 14.2 Get Person by ID

```bash
export PERSON_ID=$(curl -s "$LOOM_SERVER/api/analytics/persons?limit=1" \
  -H "Authorization: Bearer $ANALYTICS_RW_KEY" | jq -r '.persons[0].id')
curl -s "$LOOM_SERVER/api/analytics/persons/$PERSON_ID" \
  -H "Authorization: Bearer $ANALYTICS_RW_KEY" | jq .
# Expected: 200 OK with person details
```

#### 14.3 Get Person by Distinct ID

```bash
curl -s "$LOOM_SERVER/api/analytics/persons/by-distinct-id/user@example.com" \
  -H "Authorization: Bearer $ANALYTICS_RW_KEY" | jq .
# Expected: 200 OK with person details
```

#### 14.4 List Events

```bash
curl -s "$LOOM_SERVER/api/analytics/events?limit=20" \
  -H "Authorization: Bearer $ANALYTICS_RW_KEY" | jq .
# Expected: 200 OK with {events: [...], total, limit, offset}
```

#### 14.5 List Events with Filters

```bash
# By distinct_id
curl -s "$LOOM_SERVER/api/analytics/events?distinct_id=user@example.com&limit=10" \
  -H "Authorization: Bearer $ANALYTICS_RW_KEY" | jq .

# By event name
curl -s "$LOOM_SERVER/api/analytics/events?event_name=button_clicked&limit=10" \
  -H "Authorization: Bearer $ANALYTICS_RW_KEY" | jq .

# By time range
curl -s "$LOOM_SERVER/api/analytics/events?start_time=2026-01-18T00:00:00Z&end_time=2026-01-19T00:00:00Z" \
  -H "Authorization: Bearer $ANALYTICS_RW_KEY" | jq .
# Expected: 200 OK with filtered events
```

#### 14.6 Count Events

```bash
curl -s "$LOOM_SERVER/api/analytics/events/count?event_name=button_clicked" \
  -H "Authorization: Bearer $ANALYTICS_RW_KEY" | jq .
# Expected: 200 OK with {count: N}
```

#### 14.7 Export Events

```bash
curl -s -X POST "$LOOM_SERVER/api/analytics/events/export" \
  -H "Authorization: Bearer $ANALYTICS_RW_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "event_name": "button_clicked",
    "start_time": "2026-01-18T00:00:00Z",
    "limit": 1000
  }' | jq .
# Expected: 200 OK with {events: [...], total_exported}
```

#### 14.8 Query with Write-Only Key (Should Fail)

```bash
curl -s "$LOOM_SERVER/api/analytics/persons" \
  -H "Authorization: Bearer $ANALYTICS_WRITE_KEY"
# Expected: 403 Forbidden (write keys cannot query)
```

#### 14.9 Get Non-Existent Person (Should Fail)

```bash
curl -s "$LOOM_SERVER/api/analytics/persons/00000000-0000-0000-0000-000000000000" \
  -H "Authorization: Bearer $ANALYTICS_RW_KEY"
# Expected: 404 Not Found
```

---

### 15. Cross-Organization Isolation

#### 15.1 Capture from Org A, Query from Org B (Should Not See)

```bash
# Create key for Org A and capture event
# Then create key for Org B and try to query
# Expected: Events from Org A not visible to Org B
```

---

## Test Result Tracking

| # | Section | Test | Status | Notes |
|---|---------|------|--------|-------|
| 1.1 | Environments | List (empty) | | |
| 1.2 | Environments | Create | | |
| 1.3 | Environments | Get | | |
| 1.4 | Environments | Update | | |
| 1.5 | Environments | List (after create) | | |
| 1.6 | Environments | Delete | | |
| 2.1 | SDK Keys | List (empty) | | |
| 2.2 | SDK Keys | Create client-side | | |
| 2.3 | SDK Keys | Create server-side | | |
| 2.4 | SDK Keys | List (after create) | | |
| 2.5 | SDK Keys | Revoke | | |
| 3.1 | Flags | List (empty) | | |
| 3.2 | Flags | Create boolean | | |
| 3.3 | Flags | Create string variant | | |
| 3.4 | Flags | Create JSON config | | |
| 3.5 | Flags | Create with prerequisites | | |
| 3.6 | Flags | Invalid key (fail) | | |
| 3.7 | Flags | Get | | |
| 3.8 | Flags | Update | | |
| 3.9 | Flags | List (after create) | | |
| 3.10 | Flags | List including archived | | |
| 3.11 | Flags | Archive | | |
| 3.12 | Flags | Restore | | |
| 4.1 | Configs | List | | |
| 4.2 | Configs | Get | | |
| 4.3 | Configs | Enable | | |
| 4.4 | Configs | Assign strategy | | |
| 5.1 | Strategies | List (empty) | | |
| 5.2 | Strategies | Create percentage | | |
| 5.3 | Strategies | Create attribute | | |
| 5.4 | Strategies | Create geographic | | |
| 5.5 | Strategies | Create scheduled | | |
| 5.6 | Strategies | Get | | |
| 5.7 | Strategies | Update | | |
| 5.8 | Strategies | Delete | | |
| 6.1 | Kill Switches | List (empty) | | |
| 6.2 | Kill Switches | Create | | |
| 6.3 | Kill Switches | Get | | |
| 6.4 | Kill Switches | Update | | |
| 6.5 | Kill Switches | Activate | | |
| 6.6 | Kill Switches | Deactivate | | |
| 6.7 | Kill Switches | List (after create) | | |
| 6.8 | Kill Switches | Delete | | |
| 7.1 | Evaluation | Single flag | | |
| 7.2 | Evaluation | All flags | | |
| 7.3 | Evaluation | With geo | | |
| 7.4 | Evaluation | Non-existent flag (404) | | |
| 8.1 | Analytics | Flag stats | | |
| 8.2 | Analytics | Stale flags | | |
| 9.1 | Stream | Connect SSE | | |
| 9.2 | Stream | Stats (admin) | | |
| 10.1 | Admin | List platform flags | | |
| 10.2 | Admin | Create platform flag | | |
| 10.3 | Admin | List kill switches | | |
| 10.4 | Admin | Activate kill switch | | |
| 11.1 | API Keys | List (empty) | | |
| 11.2 | API Keys | Create write | | |
| 11.3 | API Keys | Create read_write | | |
| 11.4 | API Keys | List (after create) | | |
| 11.5 | API Keys | Revoke | | |
| 11.6 | API Keys | Revoke already revoked (fail) | | |
| 12.1 | Capture | Single event | | |
| 12.2 | Capture | With timestamp | | |
| 12.3 | Capture | Batch | | |
| 12.4 | Capture | $pageview | | |
| 12.5 | Capture | Invalid event name (fail) | | |
| 12.6 | Capture | Empty distinct_id (fail) | | |
| 12.7 | Capture | Empty batch (fail) | | |
| 12.8 | Capture | Batch too large (fail) | | |
| 12.9 | Capture | No auth (fail) | | |
| 12.10 | Capture | Revoked key (fail) | | |
| 13.1 | Identity | Identify | | |
| 13.2 | Identity | Alias | | |
| 13.3 | Identity | Set properties | | |
| 13.4 | Identity | Set once | | |
| 14.1 | Query | List persons | | |
| 14.2 | Query | Get person by ID | | |
| 14.3 | Query | Get person by distinct_id | | |
| 14.4 | Query | List events | | |
| 14.5 | Query | List events filtered | | |
| 14.6 | Query | Count events | | |
| 14.7 | Query | Export events | | |
| 14.8 | Query | Write key blocked (403) | | |
| 14.9 | Query | Non-existent person (404) | | |

---

## Validation History

(To be filled in during testing)
