# Systems-Theoretic Process Analysis (STPA) -- mcp-loxone

**Date:** 2026-03-09
**Scope:** Rust-based MCP server for Loxone home automation (`mcp-loxone`)
**Analyst:** STPA review of codebase at commit `1d00f20`

---

## 1. System Description and Scope

### 1.1 Purpose of Analysis

This STPA identifies safety hazards in the mcp-loxone system -- a Model Context Protocol (MCP) server that allows AI assistants and automation platforms to control physical devices in a home via a Loxone Miniserver. The system can control lights, blinds, HVAC, security alarms, door locks, intercoms, audio zones, EV chargers, and activate scenes affecting multiple devices simultaneously. Because these controls operate physical actuators affecting occupant safety, failures in the software control chain can lead to real-world harm.

### 1.2 Key Files Analyzed

- `src/server/macro_backend.rs` -- 27 MCP tool definitions (1069 lines)
- `src/server/framework_backend.rs` -- MCP protocol plumbing
- `src/client/http_client.rs` -- HTTP client with Basic Auth
- `src/client/auth.rs` -- RSA/JWT token authentication
- `src/client/mod.rs` -- LoxoneClient trait, ClientContext caching
- `src/security/input_sanitization.rs` -- Input sanitization (not integrated)
- `src/security/rate_limiting.rs` -- Rate limiting configuration
- `src/mcp_consent.rs` -- Consent management (not integrated)
- `src/config/credentials.rs` -- Credential storage
- `src/main.rs` -- Server entry point, transport selection

---

## 2. Losses

| ID | Loss | Severity |
|----|------|----------|
| **L-1** | Bodily harm to occupants (motorized blind injury, HVAC burns/hypothermia, entrapment, unauthorized entry) | Critical |
| **L-2** | Property damage (electrical fire, water damage, EV overcharge) | Critical |
| **L-3** | Physical security breach (unlocked doors, disabled alarms, opened intercom) | Critical |
| **L-4** | Privacy violation (credential theft, occupancy exposure, camera feed access) | High |
| **L-5** | Loss of environmental control (heating/cooling failure affecting vulnerable occupants) | High |
| **L-6** | Financial loss (excessive energy, unnecessary HVAC, EV overcharging) | Medium |
| **L-7** | Loss of service availability during emergency | High |

---

## 3. System-Level Hazards

| ID | Hazard | Related Losses |
|----|--------|---------------|
| **H-1** | MCP server sends command to wrong device UUID | L-1, L-2, L-3 |
| **H-2** | MCP server sends command with unsafe parameter value | L-1, L-2, L-5 |
| **H-3** | MCP server fails to authenticate/authorize a command request | L-1, L-2, L-3, L-4 |
| **H-4** | MCP server loses connection to Miniserver without detecting it | L-5, L-7 |
| **H-5** | MCP server allows disabling security system without occupant awareness | L-3 |
| **H-6** | MCP server exposes credentials through insecure channels | L-4 |
| **H-7** | MCP server executes bulk/scene operations affecting safety-critical devices | L-1, L-2 |
| **H-8** | MCP server accepts commands during degraded state (stale cache, lost connection) | L-1, L-5 |

---

## 4. Safety Constraints

| ID | Constraint | Enforces |
|----|-----------|----------|
| **SC-1** | Device UUIDs MUST be validated against cached structure before command dispatch | H-1 |
| **SC-2** | Temperature setpoints MUST be within safe range (16-28 C default) with consent for extremes | H-2 |
| **SC-3** | Authentication MUST be enforced on every command path; dev-mode MUST bind to localhost | H-3 |
| **SC-4** | Connection health MUST be verified before dispatching commands | H-4 |
| **SC-5** | Security mode changes MUST require explicit consent/confirmation | H-5 |
| **SC-6** | SSL verification MUST be enabled by default; credentials MUST NOT traverse cleartext | H-6 |
| **SC-7** | Bulk operations MUST enumerate affected devices and require consent for safety-critical ones | H-7 |
| **SC-8** | Commands MUST NOT be accepted when structure cache is stale or connection is degraded | H-8 |
| **SC-9** | All control commands MUST return actual Miniserver responses, not fabricated success | H-1, H-4 |
| **SC-10** | Input sanitization MUST be applied to all tool parameters before command construction | H-1, H-2 |

---

## 5. Control Structure

```
User/Occupant --> AI Assistant (Claude/n8n)
                      |
                 MCP tool call (stdio/HTTP)
                      |
                      v
              mcp-loxone MCP Server
              [Security Layer (exists but not integrated)]
              [Consent Manager (exists but not integrated)]
              [Tool Handlers (validate params, return fabricated success)]
              [Loxone Client (HTTP Basic Auth, retry logic)]
                      |
                 HTTP GET /jdev/sps/io/{uuid}/{command}
                      |
                      v
              Loxone Miniserver
                      |
              Physical Devices (lights, blinds, HVAC, locks, alarms)
```

### Control Actions (from MCP Server to Miniserver)

1. `control_light` -- on/off/dim (0-100%)
2. `control_blind` -- up/down/stop/position (0-100%)
3. `set_temperature` -- target temperature (5.0-35.0 C)
4. `set_security_mode` -- arm/disarm/arm_night/arm_away
5. `control_door_lock` -- lock/unlock/open
6. `control_intercom` -- answer/decline/open_door
7. `activate_scene` -- trigger named scene (multiple devices)
8. `control_audio` -- play/pause/volume (0-100%)

### Feedback (from Miniserver to MCP Server)

- HTTP response codes and JSON bodies (per-command)
- WebSocket event stream (real-time state updates -- partially implemented)
- Structure file (device inventory, cached on startup)

---

## 6. Unsafe Control Actions (UCAs)

### 6.1 Light Control

| | Not Providing | Providing Causes Hazard | Wrong Timing/Order | Stopped Too Soon |
|---|---|---|---|---|
| UCA-L1 | Lights not turned on during emergency evacuation | -- | Strobe/rapid cycling causes epileptic seizure | -- |
| UCA-L2 | -- | Light turned on in sleeping area at full brightness | -- | -- |

### 6.2 Blind Control

| | Not Providing | Providing Causes Hazard | Wrong Timing/Order | Stopped Too Soon |
|---|---|---|---|---|
| UCA-B1 | Blind not stopped when obstruction detected | Blind closes on person/pet in window frame | Blind commanded while person is leaning out | Stop command not sent/received |
| UCA-B2 | -- | All blinds closed simultaneously (fire evacuation blocked) | -- | -- |

### 6.3 Temperature/HVAC Control

| | Not Providing | Providing Causes Hazard | Wrong Timing/Order | Stopped Too Soon |
|---|---|---|---|---|
| UCA-T1 | Heating not activated in freezing conditions | Temperature set to 35 C (heat stroke risk) | -- | Heating stopped during cold snap |
| UCA-T2 | -- | Temperature set to 5 C (hypothermia risk) | -- | -- |
| UCA-T3 | -- | HVAC set to extreme while vulnerable occupant present | -- | -- |

### 6.4 Security/Alarm Control

| | Not Providing | Providing Causes Hazard | Wrong Timing/Order | Stopped Too Soon |
|---|---|---|---|---|
| UCA-S1 | Alarm not armed when occupants leave | Alarm disarmed without occupant knowledge | Alarm armed while occupants still inside | -- |
| UCA-S2 | -- | Security mode changed by unauthorized AI prompt | -- | -- |
| UCA-S3 | -- | Night mode set to away mode (different zone coverage) | -- | -- |

### 6.5 Door Lock Control

| | Not Providing | Providing Causes Hazard | Wrong Timing/Order | Stopped Too Soon |
|---|---|---|---|---|
| UCA-D1 | Door not locked when it should be | Door unlocked for unauthorized person | Door unlocked while alarm is armed | -- |
| UCA-D2 | Door not unlocked during emergency | -- | -- | -- |

### 6.6 Intercom Control

| | Not Providing | Providing Causes Hazard | Wrong Timing/Order | Stopped Too Soon |
|---|---|---|---|---|
| UCA-I1 | -- | Door opened via intercom for unknown visitor | open_door without visual/audio verification | -- |
| UCA-I2 | -- | Intercom answered exposing home audio to stranger | -- | -- |

### 6.7 Scene Activation

| | Not Providing | Providing Causes Hazard | Wrong Timing/Order | Stopped Too Soon |
|---|---|---|---|---|
| UCA-SC1 | -- | Scene affects safety devices (locks, alarms, blinds) without enumeration | Scene activated during emergency overrides prior safe state | Scene partially executed (some devices changed, others not) |

### 6.8 Audio Control

| | Not Providing | Providing Causes Hazard | Wrong Timing/Order | Stopped Too Soon |
|---|---|---|---|---|
| UCA-A1 | -- | Volume set to 100% causing hearing damage | Audio started at max volume without ramping | -- |
| UCA-A2 | -- | Audio masks smoke alarm or intrusion alert | -- | -- |

---

## 7. Loss Scenarios

### LS-1: Tool Handlers Return Fabricated Success (Critical)

**UCA:** UCA-D1, UCA-S1, UCA-T1 (all control actions)
**Code Evidence:** All control tool handlers in `macro_backend.rs` return `"status": "executed"` or `"status": "success"` without actually calling `self.get_client()?.send_command(...)`. For example, `control_door_lock` (lines 907-912) constructs a JSON response with the normalized action but never dispatches the command to the Miniserver.
**Scenario:** User asks AI to lock front door. MCP server returns success. Door remains unlocked. User leaves home believing door is secure.
**Safety Constraint Violated:** SC-9

### LS-2: Consent Management System is Dead Code (Critical)

**UCA:** UCA-S2, UCA-D1, UCA-I1
**Code Evidence:** `mcp_consent.rs` (780 lines) implements consent classification (Critical for security ops, High for bulk ops) with consent request/response flows, caching, and audit trails. None of the 27 tool implementations invoke `ConsentManager::request_consent()`.
**Scenario:** AI assistant decides to disarm alarm or unlock door based on misunderstood user intent. No confirmation step exists.
**Safety Constraint Violated:** SC-5, SC-7

### LS-3: SSL Verification Disabled by Default (Critical)

**UCA:** UCA-S2, UCA-D1 (unauthorized commands via credential theft)
**Code Evidence:** In `main.rs`, `verify_ssl` is explicitly set to `false` for all three transport modes (lines 298, 368, 394). The HTTP client (`http_client.rs` line 56-58) only logs a warning. Basic Auth sends Base64-encoded credentials in every HTTP request.
**Scenario:** On shared network (apartment building, office), attacker intercepts Basic Auth credentials, gains full device control.
**Safety Constraint Violated:** SC-6

### LS-4: No UUID Validation Against Known Devices (Critical)

**UCA:** UCA-L1, UCA-B1, UCA-T1 (wrong device)
**Code Evidence:** `send_command(uuid, command)` in `http_client.rs` constructs URLs via `format!("jdev/sps/io/{uuid}/{command}")` (line 259) without validating UUID exists in structure or matches Loxone UUID format. The input sanitization module defines a UUID whitelist pattern but is not enforced in the command path.
**Scenario:** Malformed UUID from AI parsing error controls wrong device. Blind motor commanded instead of light.
**Safety Constraint Violated:** SC-1, SC-10

### LS-5: Status Tools Return Metadata Without Live State (High)

**UCA:** UCA-T1, UCA-S1, UCA-D1 (uninformed control decisions)
**Code Evidence:** All `get_*_status` tools iterate the structure file and return device metadata (UUID, name, type, room) but never query live state values from the Miniserver.
**Scenario:** AI asks "is the alarm armed?" -- gets list of alarm devices but not their current state. Makes control decision based on assumption.
**Safety Constraint Violated:** SC-8

### LS-6: Temperature Range Allows Harmful Values (High)

**UCA:** UCA-T1, UCA-T2, UCA-T3
**Code Evidence:** `set_temperature` validates 5.0-35.0 C range. No consent required for extreme values.
**Scenario:** AI misinterprets "set it to 35" (Fahrenheit intent) as 35 C. Nursery reaches dangerous temperature.
**Safety Constraint Violated:** SC-2

### LS-7: Dev Mode Has No Network Restrictions (High)

**UCA:** UCA-S2, UCA-D1 (unauthorized access)
**Code Evidence:** `--dev-mode` flag disables authentication with no localhost binding restriction.
**Scenario:** Developer deploys with dev-mode accidentally. Any network client can control all devices without authentication.
**Safety Constraint Violated:** SC-3

### LS-8: Input Sanitization Not Integrated (High)

**UCA:** UCA-L1, UCA-B1 (command injection)
**Code Evidence:** `InputSanitizer` in `src/security/input_sanitization.rs` (908 lines) with XSS, SQL injection, path traversal prevention exists but is not invoked during MCP tool calls. Tool parameters flow directly to `format!("jdev/sps/io/{uuid}/{command}")`.
**Scenario:** Malicious input in UUID or command parameter could inject additional path segments in the Miniserver HTTP request.
**Safety Constraint Violated:** SC-10

### LS-9: Connection Health Not Verified Before Commands (High)

**UCA:** UCA-D1, UCA-S1 (silent command loss)
**Code Evidence:** `ensure_connected()` checks `self.client.is_none()` but not whether the connection is currently healthy. HTTP client does not verify connection before each command.
**Scenario:** Miniserver reboots. MCP server still has client reference. Lock/alarm commands silently fail. Tool returns fabricated success (compounded by LS-1).
**Safety Constraint Violated:** SC-4

### LS-10: Scene Activation Without Device Enumeration (Medium)

**UCA:** UCA-SC1
**Code Evidence:** `activate_scene` sends a single command without enumerating what devices the scene will affect. Scenes can include security devices, locks, and motorized blinds.
**Scenario:** AI activates "Good Night" scene which includes locking doors, closing blinds, and arming alarm. Occupant is still outside. Partial execution leaves system in inconsistent state.
**Safety Constraint Violated:** SC-7

---

## 8. Recommendations

### Critical Priority

| ID | Recommendation | Addresses |
|----|---------------|-----------|
| **R-1** | Wire tool handlers to actually call `send_command()` and return real Miniserver responses | LS-1, SC-9 |
| **R-2** | Integrate `ConsentManager` into security-critical tool paths (alarm, door lock, intercom open, bulk ops) | LS-2, SC-5 |
| **R-3** | Enable SSL verification by default; require explicit `--insecure` flag to disable | LS-3, SC-6 |
| **R-4** | Validate device UUIDs against cached structure before sending commands | LS-4, SC-1 |

### High Priority

| ID | Recommendation | Addresses |
|----|---------------|-----------|
| **R-5** | Narrow default safe temperature range to 16-28 C; require consent for values outside this range | LS-6, SC-2 |
| **R-6** | Return live device state values in status query tools (query Miniserver, not just structure cache) | LS-5, SC-8 |
| **R-7** | Restrict `--dev-mode` to localhost binding only | LS-7, SC-3 |
| **R-8** | Integrate `InputSanitizer` into the MCP tool call path before command construction | LS-8, SC-10 |
| **R-9** | Add connection health check (ping/heartbeat) in `ensure_connected()` before dispatching commands | LS-9, SC-4 |

### Medium Priority

| ID | Recommendation | Addresses |
|----|---------------|-----------|
| **R-10** | Add two-phase confirmation for irreversible actions (blind close, full scene activation) | LS-10, SC-7 |
| **R-11** | Implement emergency stop tool that halts all active actuators | L-1, L-7 |
| **R-12** | Encrypt credentials at rest in FileSystem store (addresses Issue #23) | L-4, SC-6 |
| **R-13** | Add per-device command rate limiting with minimum interval for motorized devices (blinds) | L-1, H-2 |
| **R-14** | Add structured audit logging for all successful control commands | L-4, SC-5 |

### Low Priority

| ID | Recommendation | Addresses |
|----|---------------|-----------|
| **R-15** | Add audio volume ramping (max 10% change per step) to prevent hearing damage | UCA-A1 |
| **R-16** | Check if audio playback could mask safety alarms and warn/prevent | UCA-A2 |
| **R-17** | Add Miniserver firmware version check to validate command compatibility | H-8 |
| **R-18** | Add scene enumeration tool so AI can inspect scene contents before activation | LS-10 |

---

## 9. Notes on Existing Security Infrastructure

The codebase contains substantial security infrastructure that is **implemented but not wired into the tool execution path**:

1. **ConsentManager** (`mcp_consent.rs`, 780 lines) -- Full consent flow with sensitivity classification, caching, audit trail
2. **InputSanitizer** (`security/input_sanitization.rs`, 908 lines) -- XSS, injection, path traversal prevention with blacklists
3. **RateLimiter** (`security/rate_limiting.rs`) -- Configurable rate limiting with token bucket
4. **SecurityAudit** -- Audit logging infrastructure

Integrating these existing modules (rather than writing new ones) would address findings R-2, R-8, R-13, and R-14. The code quality is high; the gap is in integration, not implementation.
