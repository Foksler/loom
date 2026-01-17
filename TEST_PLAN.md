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
        "operator": "in",
        "values": ["US", "CA"]
      }
    ]
  }' | jq .
# Expected: 201 Created
# Note: Geographic conditions require 'operator' field ("in" or "not_in")
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

This section validates that analytics data from one organization cannot be accessed by another organization.

#### 15.1 Create API Keys for Two Different Orgs

```bash
# Create read_write API key for Org A
export ANALYTICS_KEY_A=$(curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_A/analytics/api-keys" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Cross-Org Test Key A", "key_type": "read_write"}' | jq -r '.key')
echo "Key A: $ANALYTICS_KEY_A"

# Create read_write API key for Org B
export ANALYTICS_KEY_B=$(curl -s -X POST "$LOOM_SERVER/api/orgs/$ORG_B/analytics/api-keys" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Cross-Org Test Key B", "key_type": "read_write"}' | jq -r '.key')
echo "Key B: $ANALYTICS_KEY_B"
# Expected: Both keys created successfully with loom_analytics_rw_ prefix
```

#### 15.2 Capture Event for Org A

```bash
export UNIQUE_EVENT="cross_org_test_$(date +%s)"
curl -s -X POST "$LOOM_SERVER/api/analytics/capture" \
  -H "Authorization: Bearer $ANALYTICS_KEY_A" \
  -H "Content-Type: application/json" \
  -d '{
    "distinct_id": "cross-org-test-user-a",
    "event": "'"$UNIQUE_EVENT"'",
    "properties": {"test": "org_a_only"}
  }' | jq .
# Expected: 200 OK with {status: "ok", event_id: "..."}
```

#### 15.3 Verify Event Visible to Org A

```bash
curl -s "$LOOM_SERVER/api/analytics/events?event_name=$UNIQUE_EVENT" \
  -H "Authorization: Bearer $ANALYTICS_KEY_A" | jq .
# Expected: 200 OK with events array containing the captured event
```

#### 15.4 Verify Event NOT Visible to Org B

```bash
curl -s "$LOOM_SERVER/api/analytics/events?event_name=$UNIQUE_EVENT" \
  -H "Authorization: Bearer $ANALYTICS_KEY_B" | jq .
# Expected: 200 OK with {"events": [], "total": 0, ...}
# SECURITY: Org B must NOT see Org A's events
```

#### 15.5 Get Org A's Person by ID from Org B (Should Fail)

```bash
# Get person ID from Org A
export PERSON_ID=$(curl -s "$LOOM_SERVER/api/analytics/persons?limit=1" \
  -H "Authorization: Bearer $ANALYTICS_KEY_A" | jq -r '.persons[0].id')

# Try to access from Org B
curl -s "$LOOM_SERVER/api/analytics/persons/$PERSON_ID" \
  -H "Authorization: Bearer $ANALYTICS_KEY_B"
# Expected: 404 Not Found
# SECURITY: Org B must NOT access Org A's person data
```

#### 15.6 Get Org A's Person by Distinct ID from Org B (Should Fail)

```bash
curl -s "$LOOM_SERVER/api/analytics/persons/by-distinct-id/cross-org-test-user-a" \
  -H "Authorization: Bearer $ANALYTICS_KEY_B"
# Expected: 404 Not Found
# SECURITY: Org B must NOT access Org A's persons via distinct_id
```

---

## Part 3: SCM (Source Code Management)

### 16. Repository CRUD

#### 16.1 List Repos (Empty)

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/repos" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"repos": []}
```

#### 16.2 Create Repository

```bash
curl -s -X POST "$LOOM_SERVER/api/repos" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "owner_type": "org",
    "owner_id": "'"$ORG_ID"'",
    "name": "test-repo",
    "visibility": "private"
  }' | jq .
# Expected: 201 Created with repo details including clone_url
```

**Save repo ID:**
```bash
export REPO_ID=$(curl -s -X POST "$LOOM_SERVER/api/repos" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"owner_type": "org", "owner_id": "'"$ORG_ID"'", "name": "test-git-ops", "visibility": "private"}' | jq -r '.id')
export CLONE_URL=$(curl -s "$LOOM_SERVER/api/repos/$REPO_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq -r '.clone_url')
echo "REPO_ID: $REPO_ID"
echo "CLONE_URL: $CLONE_URL"
```

#### 16.3 Get Repository

```bash
curl -s "$LOOM_SERVER/api/repos/$REPO_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with repo details
```

#### 16.4 Update Repository

```bash
curl -s -X PATCH "$LOOM_SERVER/api/repos/$REPO_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "test-repo-updated", "visibility": "public"}' | jq .
# Expected: 200 OK with updated details, clone_url updated
```

#### 16.5 List Repos (After Create)

```bash
curl -s "$LOOM_SERVER/api/orgs/$ORG_ID/repos" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with repos array containing the new repo
```

#### 16.6 Invalid Repo Name (Should Fail)

```bash
curl -s -X POST "$LOOM_SERVER/api/repos" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"owner_type": "org", "owner_id": "'"$ORG_ID"'", "name": "invalid repo name", "visibility": "private"}'
# Expected: 400 Bad Request with invalid_name error
```

#### 16.7 Soft Delete Repository

```bash
curl -s -X DELETE "$LOOM_SERVER/api/repos/$REPO_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN"
# Expected: 204 No Content
```

#### 16.8 Get Deleted Repo (Should 404)

```bash
curl -s "$LOOM_SERVER/api/repos/$REPO_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN"
# Expected: 404 Not Found
```

---

### 17. Git HTTP Protocol

#### 17.1 Git info/refs (Upload-Pack)

```bash
curl -s "${CLONE_URL}/info/refs?service=git-upload-pack" \
  -H "Authorization: Bearer $LOOM_TOKEN"
# Expected: 200 OK with git protocol response showing refs
```

#### 17.2 Git info/refs (Receive-Pack)

```bash
curl -s "${CLONE_URL}/info/refs?service=git-receive-pack" \
  -H "Authorization: Bearer $LOOM_TOKEN"
# Expected: 200 OK with git protocol response showing capabilities
```

#### 17.3 Git Clone with Token Auth

```bash
git clone "https://oauth2:${LOOM_TOKEN}@loom.ghuntley.com/git/{owner}/{repo}.git" test-repo
# Expected: Clone succeeds
```

#### 17.4 Git Push with Token Auth

```bash
cd test-repo
git config user.email "test@example.com"
git config user.name "Test User"
echo "Test content" > test-file.txt
git add test-file.txt
git commit -m "Test commit"
git remote set-url origin "https://oauth2:${LOOM_TOKEN}@loom.ghuntley.com/git/{owner}/{repo}.git"
git push origin cannon
# Expected: Push succeeds
```

---

### 18. Branch Protection

#### 18.1 List Protection Rules (Empty)

```bash
curl -s "$LOOM_SERVER/api/repos/$REPO_ID/protection" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"rules": []}
```

#### 18.2 Create Protection Rule

```bash
curl -s -X POST "$LOOM_SERVER/api/repos/$REPO_ID/protection" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "pattern": "cannon",
    "block_direct_push": true,
    "block_force_push": true,
    "block_deletion": true
  }' | jq .
# Expected: 201 Created with rule details
```

**Save rule ID:**
```bash
export RULE_ID=$(curl -s -X POST "$LOOM_SERVER/api/repos/$REPO_ID/protection" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"pattern": "cannon", "block_direct_push": true, "block_force_push": true, "block_deletion": true}' | jq -r '.id')
echo "RULE_ID: $RULE_ID"
```

#### 18.3 List Protection Rules (After Create)

```bash
curl -s "$LOOM_SERVER/api/repos/$REPO_ID/protection" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with rules array
```

#### 18.4 Push to Protected Branch (Admin Bypass)

```bash
# Admins can bypass protection rules per spec
cd test-repo && git push origin cannon
# Expected: Push succeeds (admin can bypass)
```

#### 18.5 Delete Protection Rule

```bash
curl -s -X DELETE "$LOOM_SERVER/api/repos/$REPO_ID/protection/$RULE_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN"
# Expected: 204 No Content
```

---

### 19. Webhooks

#### 19.1 List Webhooks (Empty)

```bash
curl -s "$LOOM_SERVER/api/repos/$REPO_ID/webhooks" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"webhooks": []}
```

#### 19.2 Create Webhook

```bash
curl -s -X POST "$LOOM_SERVER/api/repos/$REPO_ID/webhooks" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://webhook.site/test-endpoint",
    "secret": "webhook-secret-12345",
    "payload_format": "github-compat",
    "events": ["push"]
  }' | jq .
# Expected: 201 Created with webhook details
```

**Save webhook ID:**
```bash
export WEBHOOK_ID=$(curl -s -X POST "$LOOM_SERVER/api/repos/$REPO_ID/webhooks" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"url": "https://webhook.site/test", "secret": "secret123", "payload_format": "github-compat", "events": ["push"]}' | jq -r '.id')
echo "WEBHOOK_ID: $WEBHOOK_ID"
```

#### 19.3 List Webhooks (After Create)

```bash
curl -s "$LOOM_SERVER/api/repos/$REPO_ID/webhooks" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with webhooks array
```

#### 19.4 Delete Webhook

```bash
curl -s -X DELETE "$LOOM_SERVER/api/repos/$REPO_ID/webhooks/$WEBHOOK_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN"
# Expected: 204 No Content
```

---

### 21. Push Mirrors

#### 21.1 List Push Mirrors (Empty)

```bash
curl -s "$LOOM_SERVER/api/repos/$REPO_ID/mirrors" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"mirrors": []}
```

#### 21.2 Create Push Mirror

```bash
curl -s -X POST "$LOOM_SERVER/api/repos/$REPO_ID/mirrors" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "remote_url": "https://github.com/example/mirror-target.git",
    "enabled": true
  }' | jq .
# Expected: 201 Created with {id, repo_id, remote_url, enabled, last_pushed_at, last_error, created_at}
```

**Save mirror ID:**
```bash
export MIRROR_ID=$(curl -s -X POST "$LOOM_SERVER/api/repos/$REPO_ID/mirrors" \
  -H "Authorization: Bearer $LOOM_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"remote_url": "https://github.com/example/test.git", "enabled": true}' | jq -r '.id')
echo "MIRROR_ID: $MIRROR_ID"
```

#### 21.3 List Push Mirrors (After Create)

```bash
curl -s "$LOOM_SERVER/api/repos/$REPO_ID/mirrors" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with mirrors array containing the new mirror
```

#### 21.4 Trigger Mirror Sync

```bash
curl -s -X POST "$LOOM_SERVER/api/repos/$REPO_ID/mirrors/$MIRROR_ID/sync" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"message": "Mirror sync has been queued", "queued": true}
```

#### 21.5 Delete Push Mirror

```bash
curl -s -X DELETE "$LOOM_SERVER/api/repos/$REPO_ID/mirrors/$MIRROR_ID" \
  -H "Authorization: Bearer $LOOM_TOKEN"
# Expected: 204 No Content
```

---

### 22. On-Demand Mirroring

On-demand mirroring allows cloning public GitHub/GitLab repositories through Loom. When a client
accesses a mirror URL that doesn't exist, Loom automatically:
1. Verifies the remote repository exists
2. Creates the mirror repository
3. Clones from the upstream source
4. Serves the data to the client

#### 22.1 On-Demand Mirror info/refs (New Mirror)

```bash
# First request to a mirror URL creates the mirror on-demand
curl -s "${LOOM_SERVER}/git/mirrors/github/octocat/Hello-World.git/info/refs?service=git-upload-pack" \
  -H "Authorization: Bearer $LOOM_TOKEN"
# Expected: 200 OK with git protocol response containing refs
# Note: First request may take a few seconds as the repo is cloned
```

#### 22.2 Git Clone via On-Demand Mirror

```bash
git clone "https://oauth2:${LOOM_TOKEN}@loom.ghuntley.com/git/mirrors/github/octocat/Hello-World.git"
# Expected: Clone succeeds, repository contains mirrored content
```

#### 22.3 On-Demand Mirror Non-Existent Repo (Should Fail)

```bash
curl -s "${LOOM_SERVER}/git/mirrors/github/nonexistent-user-xyz/nonexistent-repo-xyz.git/info/refs?service=git-upload-pack" \
  -H "Authorization: Bearer $LOOM_TOKEN"
# Expected: 404 Not Found with {"error": "not_found", "message": "..."}
```

#### 22.4 On-Demand Mirror Public Access (No Auth)

```bash
# On-demand mirrors of public repos are publicly accessible
curl -s "${LOOM_SERVER}/git/mirrors/github/octocat/Hello-World.git/info/refs?service=git-upload-pack"
# Expected: 200 OK with git refs (no auth required for public mirrors)
```

---

### 23. Team Access

#### 23.1 List Repo Teams (Empty)

```bash
curl -s "$LOOM_SERVER/api/repos/$REPO_ID/teams" \
  -H "Authorization: Bearer $LOOM_TOKEN" | jq .
# Expected: 200 OK with {"teams": []}
```

---

### 20. Loom CLI Credential Helper

#### 20.1 Credential Helper Get

```bash
printf "protocol=https\nhost=loom.ghuntley.com\n" | loom credential-helper get
# Expected: Returns username=oauth2 and password={token}
```

#### 20.2 Git Clone with Credential Helper

```bash
git -c credential.https://loom.ghuntley.com.helper='loom credential-helper' \
  clone https://loom.ghuntley.com/git/{owner}/{repo}.git
# Expected: Clone succeeds using stored credentials
```

---

## Test Result Tracking

| # | Section | Test | Status | Notes |
|---|---------|------|--------|-------|
| 1.1 | Environments | List (empty) | ✅ PASS | Returns environments array |
| 1.2 | Environments | Create | ✅ PASS | Validates name format (lowercase, underscores) |
| 1.3 | Environments | Get | ✅ PASS | Returns environment details |
| 1.4 | Environments | Update | ✅ PASS | Name and color updated |
| 1.5 | Environments | List (after create) | ✅ PASS | Shows new environment |
| 1.6 | Environments | Delete | ✅ PASS | Returns success message |
| 2.1 | SDK Keys | List (empty) | ✅ PASS | Returns sdk_keys array |
| 2.2 | SDK Keys | Create client-side | ✅ PASS | Returns key with loom_sdk_client_ prefix |
| 2.3 | SDK Keys | Create server-side | ✅ PASS | Returns key with loom_sdk_server_ prefix |
| 2.4 | SDK Keys | List (after create) | ✅ PASS | Shows keys (key field redacted) |
| 2.5 | SDK Keys | Revoke | ✅ PASS | Returns success message |
| 3.1 | Flags | List (empty) | ✅ PASS | Returns flags array |
| 3.2 | Flags | Create boolean | ✅ PASS | Returns flag with variants |
| 3.3 | Flags | Create string variant | ✅ PASS | Returns flag with string variants |
| 3.4 | Flags | Create JSON config | ✅ PASS | Returns flag with JSON value |
| 3.5 | Flags | Create with prerequisites | ✅ PASS | Returns flag with prerequisites array |
| 3.6 | Flags | Invalid key (fail) | ✅ PASS | Returns invalid_key error |
| 3.7 | Flags | Get | ✅ PASS | Returns full flag details |
| 3.8 | Flags | Update | ✅ PASS | Name, description, tags updated |
| 3.9 | Flags | List (after create) | ✅ PASS | Shows new flags |
| 3.10 | Flags | List including archived | ✅ PASS | Returns all flags |
| 3.11 | Flags | Archive | ✅ PASS | Returns success message |
| 3.12 | Flags | Restore | ✅ PASS | Returns success message |
| 4.1 | Configs | List | ✅ PASS | Returns configs array with environments |
| 4.2 | Configs | Get | ✅ PASS | Returns config with enabled status |
| 4.3 | Configs | Enable | ✅ PASS | Config enabled:true after update |
| 4.4 | Configs | Assign strategy | ✅ PASS | strategy_id set, evaluation uses strategy |
| 5.1 | Strategies | List (empty) | ✅ PASS | Returns strategies array |
| 5.2 | Strategies | Create percentage | ✅ PASS | Returns strategy with percentage |
| 5.3 | Strategies | Create attribute | ✅ PASS | Returns strategy with Attribute condition |
| 5.4 | Strategies | Create geographic | ✅ PASS | Returns strategy with Geographic condition (requires operator field) |
| 5.5 | Strategies | Create scheduled | ✅ PASS | Returns strategy with schedule steps |
| 5.6 | Strategies | Get | ✅ PASS | Returns full strategy details |
| 5.7 | Strategies | Update | ✅ PASS | Name and percentage updated |
| 5.8 | Strategies | Delete | ✅ PASS | Returns success message |
| 6.1 | Kill Switches | List (empty) | ✅ PASS | Returns kill_switches array |
| 6.2 | Kill Switches | Create | ✅ PASS | Returns kill switch with id, key, is_active:false |
| 6.3 | Kill Switches | Get | ✅ PASS | Returns full kill switch details |
| 6.4 | Kill Switches | Update | ✅ PASS | Name and linked_flag_keys updated |
| 6.5 | Kill Switches | Activate | ✅ PASS | is_active:true, activation_reason set |
| 6.6 | Kill Switches | Deactivate | ✅ PASS | is_active:false after deactivate |
| 6.7 | Kill Switches | List (after create) | ✅ PASS | Shows kill_switches array |
| 6.8 | Kill Switches | Delete | ✅ PASS | Returns success message |
| 7.1 | Evaluation | Single flag | ✅ PASS | Returns variant, value, reason |
| 7.2 | Evaluation | All flags | ✅ PASS | Returns results array with all flags |
| 7.3 | Evaluation | With geo | ✅ PASS | Returns results with geo context |
| 7.4 | Evaluation | Non-existent flag (404) | ✅ PASS | Returns 404 not_found |
| 8.1 | Analytics | Flag stats | ✅ PASS | Returns evaluation counts |
| 8.2 | Analytics | Stale flags | ✅ PASS | Returns stale_flags array, threshold |
| 9.1 | Stream | Connect SSE | ✅ PASS | Sends init event with all flags |
| 9.2 | Stream | Stats (admin) | ✅ PASS | Returns channel_count, total_receivers |
| 10.1 | Admin | List platform flags | ✅ PASS | Returns platform flags (org_id: null) |
| 10.2 | Admin | Create platform flag | ✅ PASS | Creates flag with org_id: null |
| 10.3 | Admin | List kill switches | ✅ PASS | Returns platform kill switches |
| 10.4 | Admin | Activate kill switch | ✅ PASS | Sets is_active:true with reason |
| 11.1 | API Keys | List (empty) | ✅ PASS | Returns api_keys array |
| 11.2 | API Keys | Create write | ✅ PASS | Returns key with loom_analytics_write_ prefix |
| 11.3 | API Keys | Create read_write | ✅ PASS | Returns key with loom_analytics_rw_ prefix |
| 11.4 | API Keys | List (after create) | ✅ PASS | Shows new keys (key redacted) |
| 11.5 | API Keys | Revoke | ✅ PASS | Returns success message |
| 11.6 | API Keys | Revoke already revoked (fail) | ✅ PASS | Returns 400 already_revoked |
| 12.1 | Capture | Single event | ✅ PASS | Returns status:ok, event_id |
| 12.2 | Capture | With timestamp | ✅ PASS | Returns status:ok, event_id |
| 12.3 | Capture | Batch | ✅ PASS | Returns status:ok, count:3 |
| 12.4 | Capture | $pageview | ✅ PASS | Returns status:ok, event_id |
| 12.5 | Capture | Invalid event name (fail) | ✅ PASS | Returns invalid_event_name error |
| 12.6 | Capture | Empty distinct_id (fail) | ✅ PASS | Returns invalid_distinct_id error |
| 12.7 | Capture | Empty batch (fail) | ✅ PASS | Returns empty_batch error |
| 12.8 | Capture | Batch too large (fail) | ✅ PASS | Returns 400 batch_too_large |
| 12.9 | Capture | No auth (fail) | ✅ PASS | Returns 401 unauthorized |
| 12.10 | Capture | Revoked key (fail) | ✅ PASS | Returns 401 unauthorized |
| 13.1 | Identity | Identify | ✅ PASS | Returns status:ok, person_id |
| 13.2 | Identity | Alias | ✅ PASS | Returns status:ok, person_id |
| 13.3 | Identity | Set properties | ✅ PASS | Returns status:ok, person_id |
| 13.4 | Identity | Set once | ✅ PASS | Returns status:ok, person_id (sets only if not exists) |
| 14.1 | Query | List persons | ✅ PASS | Returns persons array, total, limit |
| 14.2 | Query | Get person by ID | ✅ PASS | Returns person with properties, identities |
| 14.3 | Query | Get person by distinct_id | ✅ PASS | Returns person details |
| 14.4 | Query | List events | ✅ PASS | Returns events array, total, limit |
| 14.5 | Query | List events filtered | ✅ PASS | Returns filtered events by distinct_id |
| 14.6 | Query | Count events | ✅ PASS | Returns count |
| 14.7 | Query | Export events | ✅ PASS | Returns events with total_exported |
| 14.8 | Query | Write key blocked (403) | ✅ PASS | Returns 403 forbidden |
| 14.9 | Query | Non-existent person (404) | ✅ PASS | Returns 404 not_found |
| 15.1 | Cross-Org | Create keys for two orgs | ✅ PASS | Both keys created with loom_analytics_rw_ prefix |
| 15.2 | Cross-Org | Capture event for Org A | ✅ PASS | Returns status:ok, event_id |
| 15.3 | Cross-Org | Event visible to Org A | ✅ PASS | Returns event in events array |
| 15.4 | Cross-Org | Event NOT visible to Org B | ✅ PASS | Returns empty events array (isolation works) |
| 15.5 | Cross-Org | Person by ID (cross-org 404) | ✅ PASS | Returns 404 not_found (security isolation) |
| 15.6 | Cross-Org | Person by distinct_id (cross-org 404) | ✅ PASS | Returns 404 not_found (security isolation) |
| 16.1 | Repos | List (empty) | ✅ PASS | Returns repos array |
| 16.2 | Repos | Create | ✅ PASS | Returns id, clone_url, default_branch=cannon |
| 16.3 | Repos | Get | ✅ PASS | Returns repo details |
| 16.4 | Repos | Update | ✅ PASS | Name, visibility updated, clone_url changes |
| 16.5 | Repos | List (after create) | ✅ PASS | Shows new repo |
| 16.6 | Repos | Invalid name (fail) | ✅ PASS | Returns invalid_name error |
| 16.7 | Repos | Soft delete | ✅ PASS | Returns 204 No Content |
| 16.8 | Repos | Get deleted (404) | ✅ PASS | Returns 404 not_found |
| 17.1 | Git | info/refs upload-pack | ✅ PASS | Returns refs with capabilities |
| 17.2 | Git | info/refs receive-pack | ✅ PASS | Returns refs with capabilities |
| 17.3 | Git | Clone with token | ✅ PASS | Clone succeeds via HTTP auth |
| 17.4 | Git | Push with token | ✅ PASS | Push succeeds via HTTP auth |
| 18.1 | Protection | List (empty) | ✅ PASS | Returns rules array |
| 18.2 | Protection | Create | ✅ PASS | Returns rule details |
| 18.3 | Protection | List (after create) | ✅ PASS | Shows new rule |
| 18.4 | Protection | Push (admin bypass) | ✅ PASS | Admin can push to protected branch |
| 18.5 | Protection | Delete | ✅ PASS | Returns 204 No Content |
| 19.1 | Webhooks | List (empty) | ✅ PASS | Returns webhooks array |
| 19.2 | Webhooks | Create | ✅ PASS | Returns webhook id, enabled=true |
| 19.3 | Webhooks | List (after create) | ✅ PASS | Shows new webhook |
| 19.4 | Webhooks | Delete | ✅ PASS | Returns 204 No Content |
| 21.1 | Push Mirrors | List (empty) | ✅ PASS | Returns mirrors array |
| 21.2 | Push Mirrors | Create | ✅ PASS | Returns mirror id, enabled=true |
| 21.3 | Push Mirrors | List (after create) | ✅ PASS | Shows new mirror |
| 21.4 | Push Mirrors | Trigger sync | ✅ PASS | Returns queued=true message |
| 21.5 | Push Mirrors | Delete | ✅ PASS | Returns 204 No Content |
| 22.1 | On-demand Mirror | info/refs (new mirror) | ✅ PASS | Creates mirror, returns git refs |
| 22.2 | On-demand Mirror | Git clone | ✅ PASS | Clone succeeds, repo mirrored |
| 22.3 | On-demand Mirror | Non-existent repo (404) | ✅ PASS | Returns 404 not_found |
| 22.4 | On-demand Mirror | Public access (no auth) | ✅ PASS | Mirrors accessible without auth |
| 23.1 | Team Access | List (empty) | ✅ PASS | Returns teams array |
| 20.1 | Cred Helper | Get | ✅ PASS | Returns oauth2/token credentials |
| 20.2 | Cred Helper | Git clone | ✅ PASS | Clone succeeds with helper |

---

## Validation History

### 2026-01-18 - Cross-Organization Isolation Validation

**Tester:** Claude (automated validation)
**Server:** https://loom.ghuntley.com
**Method:** curl + loom-cli token

**Summary:**
- **Cross-Organization Isolation (15.1-15.6):** All 6 tests pass

**Total:** 6 tests validated, all passing

**Validation Steps:**
1. Created read_write API keys for two different organizations (Org A and Org B)
2. Captured a unique event (`cross_org_test_1768669247`) for Org A with distinct_id `cross-org-test-user-a`
3. Verified the event is visible when querying with Org A's API key - returns 1 event
4. Verified the event is NOT visible when querying with Org B's API key - returns 0 events
5. Retrieved Org A's person ID and attempted to access via Org B's key - returns 404 Not Found
6. Attempted to lookup person by distinct_id from Org B - returns 404 Not Found

**Security Validation:**
- ✅ Events are isolated per organization
- ✅ Persons are isolated per organization
- ✅ Person lookup by ID is isolated
- ✅ Person lookup by distinct_id is isolated
- ✅ No cross-organization data leakage detected

**Notes:**
- Cross-organization isolation is properly enforced at the database level via `org_id` filtering
- API keys are scoped to their creating organization
- All query endpoints respect organization boundaries

---

### 2026-01-18 - Mirroring System Validation

**Tester:** Claude (automated validation)
**Server:** https://loom.ghuntley.com
**Method:** curl + loom-cli token + git CLI

**Summary:**
- **Push Mirrors (21.1-21.5):** All 5 tests pass
- **On-demand Mirroring (22.1-22.4):** All 4 tests pass
- **Team Access (23.1):** Test passes

**Total:** 10 tests validated, all passing

**Validation Steps:**
1. Created test repository for push mirror testing
2. Tested list mirrors (empty) - returns `{"mirrors": []}`
3. Created push mirror with `remote_url` and `enabled: true`
4. Verified mirror appears in list with correct fields
5. Triggered mirror sync - returns `{"message": "Mirror sync has been queued", "queued": true}`
6. Deleted mirror - returns 204 No Content
7. Tested on-demand mirroring via info/refs endpoint for `mirrors/github/octocat/Hello-World.git`
   - First request triggers clone from GitHub
   - Returns git protocol refs within ~3 seconds
8. Tested git clone via on-demand mirror URL - clone succeeds
9. Tested non-existent repo returns 404 with error message
10. Verified on-demand mirrors are publicly accessible (no auth required)
11. Tested team access list endpoint - returns `{"teams": []}`

**Notes:**
- On-demand mirroring automatically creates a `mirrors` org to house mirrored repositories
- Mirrors are stored at paths like `/git/mirrors/github/{owner}/{repo}.git`
- Non-existent repos return 404 with message "Remote repository not found"
- Push mirrors support scheduled sync and on-push triggers
- Credential helper works correctly: returns `username=oauth2` and `password={token}`

---

### 2026-01-18 - Test 4.4 (Assign Strategy) Validation

**Tester:** Claude (automated validation)
**Server:** https://loom.ghuntley.com
**Method:** curl + loom-cli token

**Summary:**
- **Test 4.4 (Configs - Assign strategy):** PASS

**Validation Steps:**
1. Created environment `testing_strategy`
2. Created strategy `Test 50% Rollout` with 50% percentage rollout
3. Created flag `test.strategy_assignment` with boolean variants
4. Verified flag config initially had `strategy_id: null`
5. Assigned strategy via PATCH `/api/orgs/{org}/flags/{flag}/configs/{env}`
6. Verified `strategy_id` was set in response and persisted
7. Enabled flag and evaluated for 10 different users
8. Confirmed strategy applies correctly (some users get `variant: "on"` with `reason.type: "Strategy"`, others get default)

**Notes:**
- Flag evaluation correctly returns `reason.type: "Strategy"` when user falls within rollout percentage
- Users outside rollout get `reason.type: "Default"` with the default variant

---

### 2026-01-18 - SCM (Source Code Management) Validation

**Tester:** Claude (automated validation)
**Server:** https://loom.ghuntley.com
**Method:** curl + loom-cli token + git CLI

**Summary:**
- **Repositories (16.1-16.8):** All 8 tests pass
- **Git HTTP Protocol (17.1-17.4):** All 4 tests pass
- **Branch Protection (18.1-18.5):** All 5 tests pass
- **Webhooks (19.1-19.4):** All 4 tests pass
- **Credential Helper (20.1-20.2):** Both tests pass

**Total:** 23 tests validated, all passing

**Notes:**
- Branch protection allows admin bypass by design (see spec: "Check if pusher has repo:admin role (admins can bypass)")
- Webhook creation requires `secret` field (HMAC-SHA256 for signature verification)
- Git operations use Basic auth with `oauth2:{token}` or credential helper
- Default branch is `cannon` (not `main` or `master`)

---

### 2026-01-18 - Complete Feature Flags & Analytics Validation

**Tester:** Claude (automated validation)
**Server:** https://loom.ghuntley.com
**Method:** curl + loom-cli token

**Summary:**
- **Environments (1.1-1.6):** All 6 tests pass
- **SDK Keys (2.1-2.5):** All 5 tests pass
- **Flags (3.1-3.12):** All 12 tests pass
- **Configs (4.1-4.4):** All 4 tests pass
- **Strategies (5.1-5.8):** All 8 tests pass
- **Kill Switches (6.1-6.8):** All 8 tests pass
- **Evaluation (7.1-7.4):** All 4 tests pass
- **Flag Analytics (8.1-8.2):** Both tests pass
- **SSE Streaming (9.1-9.2):** Both tests pass
- **Admin Endpoints (10.1-10.4):** All 4 tests pass
- **Analytics API Keys (11.1-11.6):** All 6 tests pass
- **Event Capture (12.1-12.10):** All 10 tests pass
- **Identity (13.1-13.4):** All 4 tests pass
- **Query (14.1-14.9):** All 9 tests pass

**Total:** 79 tests validated, all passing

**Notes:**
- Test 5.4 (Geographic Strategy): Test plan example needed correction - Geographic conditions require `operator` field ("in"/"not_in")
- Platform admin endpoints require `is_system_admin = true` on user

---

### 2026-01-17 - Feature Flags & Analytics Validation

**Tester:** Claude (automated validation)
**Server:** https://loom.ghuntley.com
**Method:** curl + loom-cli token

**Summary:**
- **Environments (1.1-1.6):** All 6 tests pass
- **SDK Keys (2.1-2.2):** 2 of 5 tests validated, all pass
- **Flags (3.1-3.12):** 10 of 12 tests validated, all pass
- **Configs (4.1-4.3):** 3 of 4 tests validated, all pass
- **Strategies (5.1-5.2):** 2 of 8 tests validated, all pass
- **Kill Switches (6.1-6.6):** 6 of 8 tests validated, all pass
- **Evaluation (7.1-7.4):** 3 of 4 tests validated, all pass
- **Flag Analytics (8.1-8.2):** Both tests pass
- **Analytics API Keys (11.1-11.4):** 4 of 6 tests validated, all pass
- **Event Capture (12.1-12.9):** 8 of 10 tests validated, all pass
- **Identity (13.1-13.3):** 3 of 4 tests validated, all pass
- **Query (14.1, 14.4, 14.6, 14.8):** 4 of 9 tests validated, all pass

**Total:** 54 tests validated, all passing
