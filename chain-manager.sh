#!/bin/bash
# ============================================================
# P:L:I:M:/Chain Network Manager
# Usage: ./chain-manager.sh [dev|testnet|mainnet] [start|stop|status|info]
# ============================================================

set -euo pipefail

NETWORK="${1:-}"
ACTION="${2:-status}"
BINARY="/mnt/data/cargo-target/release/plim-node"
SERVICE="plim-node"
SERVICE_FILE="/etc/systemd/system/${SERVICE}.service"
CHAIN_DIR="/opt/plimlab/plim-protocol/plim-chain"
RPC_PORT=9944
RPC_URL="http://127.0.0.1:${RPC_PORT}"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m'

usage() {
    echo ""
    echo -e "${CYAN}P:L:I:M:/Chain Network Manager${NC}"
    echo ""
    echo "Usage: $0 <network> <action>"
    echo ""
    echo "Networks:"
    echo "  dev       Development mode (ephemeral --tmp storage)"
    echo "  testnet   Testnet mode (persistent at /mnt/data/plim-chain-testnet)"
    echo "  mainnet   Mainnet mode (persistent at /mnt/data/plim-chain-mainnet)"
    echo ""
    echo "Actions:"
    echo "  start     Stop current node, switch to network, start"
    echo "  stop      Stop the running node"
    echo "  status    Show service status + RPC health"
    echo "  info      Show chain info via RPC"
    echo ""
    echo "Examples:"
    echo "  $0 dev start       # Start in dev mode"
    echo "  $0 testnet start   # Switch to testnet"
    echo "  $0 mainnet start   # Switch to mainnet"
    echo "  $0 dev status      # Check current status"
    echo ""
    exit 1
}

rpc_call() {
    local method="$1"
    curl -s -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"${method}\",\"params\":[],\"id\":1}" \
        "${RPC_URL}" 2>/dev/null
}

get_exec_start() {
    local net="$1"
    case "$net" in
        dev)
            echo "${BINARY} --dev --rpc-cors=all --rpc-port=${RPC_PORT} --tmp --prometheus-external"
            ;;
        testnet)
            echo "${BINARY} --chain local --name Plim-Testnet-1 --rpc-cors=all --rpc-port=${RPC_PORT} --prometheus-external --base-path /mnt/data/plim-chain-testnet --validator"
            ;;
        mainnet)
            echo "${BINARY} --chain ${CHAIN_DIR}/chain-spec-mainnet-raw.json --name Plim-Mainnet-1 --rpc-cors=all --rpc-port=${RPC_PORT} --rpc-methods Safe --prometheus-external --base-path /mnt/data/plim-chain-mainnet --validator"
            ;;
    esac
}

write_service_file() {
    local net="$1"
    local exec_start
    exec_start=$(get_exec_start "$net")

    local description
    case "$net" in
        dev)      description="Plim Chain Node - Development" ;;
        testnet)  description="Plim Chain Node - Testnet" ;;
        mainnet)  description="Plim Chain Node - Mainnet" ;;
    esac

    sudo tee "${SERVICE_FILE}" > /dev/null << SVCEOF
[Unit]
Description=${description}
After=network.target

[Service]
Type=simple
User=plimadmin
ExecStart=${exec_start}
Restart=on-failure
RestartSec=10
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
SVCEOF
}

do_start() {
    local net="$1"
    echo -e "${YELLOW}Stopping current node...${NC}"
    sudo systemctl stop "${SERVICE}" 2>/dev/null || true
    sleep 1

    echo -e "${CYAN}Writing service file for ${net}...${NC}"
    write_service_file "$net"

    echo -e "${CYAN}Reloading systemd...${NC}"
    sudo systemctl daemon-reload

    echo -e "${GREEN}Starting P:L:I:M:/Chain (${net})...${NC}"
    sudo systemctl start "${SERVICE}"
    sleep 3

    if systemctl is-active --quiet "${SERVICE}"; then
        echo -e "${GREEN}Node started successfully on ${net}${NC}"
        do_info
    else
        echo -e "${RED}Node failed to start. Check: journalctl -u ${SERVICE} -n 20${NC}"
        exit 1
    fi
}

do_stop() {
    echo -e "${YELLOW}Stopping P:L:I:M:/Chain...${NC}"
    sudo systemctl stop "${SERVICE}"
    echo -e "${GREEN}Node stopped${NC}"
}

do_status() {
    echo -e "${CYAN}P:L:I:M:/Chain Service Status:${NC}"
    echo "---"
    systemctl status "${SERVICE}" --no-pager 2>/dev/null | head -12 || echo -e "${RED}Service not found${NC}"
    echo ""

    echo -e "${CYAN}RPC Health:${NC}"
    local health
    health=$(rpc_call "system_health" 2>/dev/null)
    if [ -n "$health" ] && echo "$health" | python3 -m json.tool 2>/dev/null; then
        :
    else
        echo -e "${RED}Node not responding on ${RPC_URL}${NC}"
    fi
}

do_info() {
    echo ""
    echo -e "${CYAN}P:L:I:M:/Chain Info:${NC}"
    echo "---"

    local chain_name
    chain_name=$(rpc_call "system_chain" 2>/dev/null)
    if [ -n "$chain_name" ]; then
        echo -n "Chain:   "; echo "$chain_name" | python3 -c "import sys,json; print(json.load(sys.stdin).get('result','?'))" 2>/dev/null
    fi

    local node_name
    node_name=$(rpc_call "system_name" 2>/dev/null)
    if [ -n "$node_name" ]; then
        echo -n "Node:    "; echo "$node_name" | python3 -c "import sys,json; print(json.load(sys.stdin).get('result','?'))" 2>/dev/null
    fi

    local version
    version=$(rpc_call "system_version" 2>/dev/null)
    if [ -n "$version" ]; then
        echo -n "Version: "; echo "$version" | python3 -c "import sys,json; print(json.load(sys.stdin).get('result','?'))" 2>/dev/null
    fi

    local health
    health=$(rpc_call "system_health" 2>/dev/null)
    if [ -n "$health" ]; then
        echo -n "Peers:   "; echo "$health" | python3 -c "import sys,json; h=json.load(sys.stdin).get('result',{}); print(h.get('peers','?'))" 2>/dev/null
        echo -n "Syncing: "; echo "$health" | python3 -c "import sys,json; h=json.load(sys.stdin).get('result',{}); print(h.get('isSyncing','?'))" 2>/dev/null
    else
        echo -e "${RED}Node not responding on ${RPC_URL}${NC}"
    fi
    echo ""
}

# --- Main ---

if [ -z "$NETWORK" ]; then
    # No network given: just show status
    do_status
    exit 0
fi

case "$NETWORK" in
    dev|testnet|mainnet) ;;
    -h|--help|help) usage ;;
    *)
        echo -e "${RED}Unknown network: ${NETWORK}${NC}"
        usage
        ;;
esac

case "$ACTION" in
    start)  do_start "$NETWORK" ;;
    stop)   do_stop ;;
    status) do_status ;;
    info)   do_info ;;
    *)
        echo -e "${RED}Unknown action: ${ACTION}${NC}"
        usage
        ;;
esac
