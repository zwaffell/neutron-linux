# Recovered Neutron MIDI protocol

Observed in the bundled Neutron 1.0.3 renderer. The application statements were extracted with a JavaScript AST parser and formatted; they are derived code, not original developer sources. Each statement records its original half-open UTF-8 byte range. The JSON map was extracted from literal AST nodes without executing the bundle. Hardware semantics remain unverified.

## Message framing

Outgoing messages: `F0 00 20 32 28 7F <command> <arguments...> F7`. Manufacturer bytes are `00 20 32`; family is `28`; the app always sends `7F` in byte 5. The meaning of that fixed byte is not established by this inspection.

Incoming parsing checks framing, manufacturer and family, reads the command at byte 6, the communications/version byte at byte 7, and data from byte 8 to the byte before `F7`. It does not validate byte 5 in the recovered recognizer.

Examples observed through the offline renderer:
```text
F0 00 20 32 28 7F 05 F7          Read global settings
F0 00 20 32 28 7F 73 F7          Request software version
F0 00 20 32 28 7F 0A 0E 01 F7    Enable oscillator sync
F0 00 20 32 28 7F 0A 26 02 F7    Set oscillator 1 range to enum 2
```

Before connection, only version and settings requests pass the sender gate. The app polls global settings every 1000 ms. On the first recognized response it requests software version if disconnected; a version response establishes connected/outdated state and requests settings again. The firmware class requires 2.0.2 or later, with major-version-zero builds also accepted by the original code. Firmware strings are parsed permissively by the original app.

## Commands

Command names below are definitions in the bundle. Definition alone does not prove that a UI action sends the command or establish destructive-command behavior.

| Name | Hex | Decimal |
|---|---:|---:|
| GLOBAL_SETTING_REQUEST | `05` | 5 |
| GLOBAL_SETTING_RESPONSE | `06` | 6 |
| PARAMETER_SET | `0A` | 10 |
| PARAMETER_SET_RESPONSE | `5A` | 90 |
| RESTORE_DEFAULT_SETTINGS | `0B` | 11 |
| CALIBRATION_MODE | `10` | 16 |
| APP_TUNING_REQUEST | `71` | 113 |
| APP_TUNING_RESPONSE | `72` | 114 |
| SOFTWARE_VER_REQUEST | `73` | 115 |
| SOFTWARE_VER_RESPONSE | `74` | 116 |
| FACTORY_RESET | `77` | 119 |

## Parameter addresses and settings layout

Writes use command `0A`, address, and one 7-bit value for scalar/boolean values; array writes use address, element index, and 7-bit element value. Boolean true is 1. The app masks integer values with `7F`. Reset-key-range and reset-LFO-order are action addresses, not normal settings. The note-priority address is shared: the paraphonic flag selects which store key is updated by its response.

The global settings decoder reconstructs 10 little-endian 16-bit words. `block`, `offset` and `length` below refer to bitfields in these words. Only blocks 0–7 have named fields here; meanings of the remaining words are unknown. “Store initial value” is the renderer’s initialization, not a measured hardware factory default.

| Parameter | Address hex | Block | Bit offset | Width | Store initial value |
|---|---:|---:|---:|---:|---|
| MIDI_CHANNEL | `00` | 1 | 9 | 4 | `0` |
| NOTE_PRIORITY | `01` | 1 | 14 | 2 | `2` |
| PARA_NOTE_PRIORITY | `01` | 7 | 12 | 2 | `2` |
| PITCH_BEND_RANGE | `03` | 2 | 0 | 5 | `2` |
| ASSIGN_OUT | `04` | 0 | 10 | 3 | `0` |
| ENV_RETRIGGER | `05` | 1 | 0 | 1 | `false` |
| RESET_KEY_RANGE | `06` | — | — | — | `action` |
| POLYCHAIN_MODE | `08` | 1 | 3 | 2 | `false` |
| DEVICE_ID | `09` | 1 | 5 | 4 | `0` |
| DISABLE_MIDI_DIPS | `0A` | 1 | 13 | 1 | `false` |
| KEY_RANGE_MUTE | `0B` | 5 | 15 | 1 | `false` |
| KEY_RANGE_MIN | `0C` | 6 | 0 | 6 | `24` |
| KEY_RANGE_MAX | `0D` | 6 | 6 | 6 | `96` |
| OSC_SYNC | `0E` | 0 | 4 | 1 | `false` |
| PARAPHONIC_MODE | `0F` | 0 | 7 | 1 | `false` |
| VCF_MODE | `10` | 0 | 8 | 2 | `1` |
| VCF_KEY_TRACK | `11` | 0 | 6 | 1 | `false` |
| VCF_MOD_SRC | `12` | 6 | 12 | 2 | `0` |
| VCF_MOD_DEPTH | `14` | 7 | 0 | 6 | `63` |
| OSC1_BLEND_MODE | `20` | 0 | 13 | 1 | `false` |
| OSC2_BLEND_MODE | `21` | 0 | 14 | 1 | `false` |
| OSC1_TUNE_POT | `22` | 1 | 1 | 1 | `false` |
| OSC2_TUNE_POT | `23` | 1 | 2 | 1 | `false` |
| OSC1_AUTOGLIDE | `24` | 2 | 5 | 5 | `12` |
| OSC2_AUTOGLIDE | `25` | 2 | 10 | 5 | `12` |
| OSC1_RANGE | `26` | 0 | 0 | 2 | `0` |
| OSC2_RANGE | `27` | 0 | 2 | 2 | `0` |
| OSC_KEY_SPLIT | `28` | 4 | 10 | 6 | `0` |
| OSC2_KEY_TRACK | `2A` | 2 | 15 | 1 | `false` |
| LFO_BLEND_MODE | `30` | 0 | 15 | 1 | `false` |
| LFO_ONE_SHOT | `31` | 6 | 14 | 1 | `false` |
| LFO_KEY_TRACK | `32` | 4 | 0 | 7 | `0` |
| LFO_DEPTH | `34` | 7 | 6 | 6 | `63` |
| LFO_MIDI_SYNC | `35` | 3 | 15 | 1 | `false` |
| LFO_KEY_SYNC | `37` | 0 | 5 | 1 | `false` |
| LFO_SHAPE_ORDER | `38` | 3 | 0 | 15 | `[0, 1, 2, 3, 4]` |
| RESET_LFO_ORDER | `39` | — | — | — | `action` |
| LFO_PHASE_OFFSET | `3A` | 5 | 0 | 15 | `[0, 0, 0, 0, 0]` |
| LFO_RETRIGGER | `3B` | 6 | 15 | 1 | `false` |

Additional decoded-value transforms: `OSC_KEY_SPLIT` is 0 when disabled, otherwise raw+23; `KEY_RANGE_MIN` is raw+24; `KEY_RANGE_MAX` is 96−raw. Array bitfields are divided equally among their elements, least-significant element first.

## SysEx binary packing and tuning

Each seven binary bytes becomes eight SysEx bytes: the seven low 7-bit parts followed by one byte containing their high bits in positions 0–6. The original encoder pads with `7 - (length % 7)` zero bytes, including an entire extra group for already aligned input. Decoder input must be entirely 7-bit and a multiple of eight bytes; decoded padding is not automatically removed. Browser tests cover high-bit roundtrip and the aligned padding behavior.

Tuning response `72` is decoded with the same packing, split into seven-byte records, and the first four bytes of each record are interpreted as a little-endian period in microseconds. The first two periods feed oscillator 1/2, and byte 4 of the first record is the note number. Frequency is `1 / (period / 1e6)`. The offline preview does not simulate tuning telemetry. Its real-hardware timing and trigger behavior remain unverified.

## Source locations

- `recovered/application-statements.js:39`: firmware requirement getter.
- `recovered/application-statements.js:441`: initial parameter store.
- `recovered/application-statements.js:733`: framing, commands and addresses.
- `recovered/application-statements.js:906`: SysEx packing and unpacking.
- `recovered/application-statements.js:1001`: settings layout and value transforms.
- `recovered/application-statements.js:1097`: incoming message handler.
- `recovered/application-statements.js:1271`: little-endian conversion and sender.
- `recovered/application-statements.js:1303`: Web MIDI acquisition and polling.
- `recovered/application-statements.js:1369`: original Windows port-name filter.
- `recovered/application-statements.js:1381`: UI write actions.
- `recovered/protocol.json`: machine-readable literal map.
- `recovered/modules.json`: 819 webpack factory identities, ranges and hashes. Factory numbers are bundle-local, not original module names.

REA renderer analysis: `ev_8bd197880fd26985a117b1a266190d66962d29db37090fefdd803a47399165c7`; summary `ev_4e030917be52d96bf1020488cc6f819a71ca9bd5671e5d2b4158ff5bf8d7a575`. It recovered 819 webpack modules and parsed 196,320 AST nodes. Coverage is partial: other files were outside this request and semantic nodes were capped at 1,500. The protocol table also relies on direct AST/source inspection; no complete semantic reconstruction is claimed.

REA final offline browser capture: `ev_642940b6dba600eb04bad7083dd30f031356d81331f95c84f3b9219d8d9ff9c4`. It observed the simulated 2.0.2 handshake, local resources and no console errors or warnings. It does not establish real-device compatibility.
