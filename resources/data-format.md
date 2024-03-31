# Websocket data format

A data packet is 5 16-bit numbers (10 bytes): `[SAVE, COLD, WARM, X, Y]`

|  Name  | meaning |
|--------|---------|
| `SAVE` | `1` if the values should be saved, `0` if they should not |
| `COLD` | PWM value for the cold LED |
| `WARM` | PWM value for the warm LED |
| `X`    | X position of the selection disk (for clients) |
| `Y`    | Y position of the selection disk (for clients) |

Each 16 bit number is represented in little endian format, for example the 16 bit value `0x1122` would be represented in two bytes as `0x22 0x11`. 

# Websocket behavior

- The websocket is located at `ws://<IP OF ESP32>:81/` as defined in [line 85 of index.html](resources/index.html#85).
- There is one global data packet in ram that must be synchronized among all clients.
- This global packet should be stored to flash and loaded into ram on startup.
- When a new client connects, the global packet in ram must be sent to them.
- When a client sends a packet, the global packet in ram must be overwritten and all clients *except the sending one* must recieve this new packet.
- When a client sends a packet where `SAVE` is `1`, the full packet must be written to flash.
