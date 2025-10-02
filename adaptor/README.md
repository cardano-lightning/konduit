# Adaptor

## Installing

```
yarn
```

## Configuring

Configuration happens through environment variables and a few local files:

```bash
# ===== REQUIRED =====

# Public URL to connect to the BLN node.
KONDUIT_ADAPTOR_LN_BASE_URL = https://127.0.0.1:8080

# Admin Macaroon file to authorize connection to the BLN node.
KONDUIT_ADAPTOR_LN_MACAROON = /path/to/admin.macaroon

# TLS certificate (PEM format) to *securily* connect to the BLN node.
KONDUIT_ADAPTOR_LN_TLS_CERT = /path/to/tls.cert

# ===== Optional =====

# TCP port to listen to for incoming client connections to the Adaptor server
KONDUIT_ADAPTOR_LISTEN_PORT = 4444,

# Fixed amount charged by the Adaptor for routing payments, in milli-satoshis
KONDUIT_ADAPTOR_FEE = 42,
```

## Running

```
yarn start
```
