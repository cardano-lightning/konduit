# Adaptor

## Specifications


## CBOR-RPC 1.0

```cddl
request = #6.49(
  {   0: version                ; CBOR-RPC version
  ,   1: text .size (0 .. 64)   ; Method
  , ? 2: any                    ; Parameters
  , ? 3: any                    ; State
  }
)

response_ok = #6.50(
  {   0: version                ; CBOR-RPC version
  ,   1: text .size (0 ..64)    ; Method
  ,   2: any                    ; Result
  , ? 3: any                    ; Mirrored-state
  }
)

response_ko = #6.51(
  {   0: version                ; CBOR-RPC version
  , ? 1: text .size (0 ..64)    ; Method
  ,   2: error                  ; Error
  , ? 3: any                    ; Mirrored-state
  }
)

error =
  {   0: (-32768..32767)        ; Error code
  , ? 1: text                   ; Message
  , ? 2: any                    ; Details
  }

version = 1
```
