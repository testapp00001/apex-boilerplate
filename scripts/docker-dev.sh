#!/bin/bash
# =============================================================================
# Apex Docker Development Helper Script
# =============================================================================
# Usage:
#   ./scripts/docker-dev.sh          # Start all services
#   ./scripts/docker-dev.sh build    # Build and start
#   ./scripts/docker-dev.sh down     # Stop services
#   ./scripts/docker-dev.sh logs     # Follow logs
#   ./scripts/docker-dev.sh migrate  # Run migrations
#   ./scripts/docker-dev.sh shell    # Shell into API container
#   ./scripts/docker-dev.sh psql     # Connect to PostgreSQL
# =============================================================================

set -e

COMPOSE_CMD="docker compose"
COMPOSE_FILE="docker-compose.yml"
PROJECT_NAME="apex"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

case "${1:-up}" in
    up)
        log_info "Starting Apex development environment..."
        $COMPOSE_CMD -f $COMPOSE_FILE -p $PROJECT_NAME up -d
        log_info "Services started. API available at http://localhost:8080"
        log_info "PostgreSQL available at localhost:5432"
        log_info "Redis available at localhost:6379"
        ;;
    
    build)
        log_info "Building and starting services..."
        $COMPOSE_CMD -f $COMPOSE_FILE -p $PROJECT_NAME up -d --build
        log_info "Build complete. API available at http://localhost:8080"
        ;;
    
    down)
        log_info "Stopping services..."
        $COMPOSE_CMD -f $COMPOSE_FILE -p $PROJECT_NAME down
        log_info "Services stopped."
        ;;
    
    clean)
        log_warn "Stopping services and removing volumes..."
        $COMPOSE_CMD -f $COMPOSE_FILE -p $PROJECT_NAME down -v
        log_info "Services stopped and volumes removed."
        ;;
    
    logs)
        $COMPOSE_CMD -f $COMPOSE_FILE -p $PROJECT_NAME logs -f ${2:-api}
        ;;
    
    migrate)
        log_info "Running database migrations..."
        $COMPOSE_CMD -f $COMPOSE_FILE -p $PROJECT_NAME exec api ./migration up
        log_info "Migrations complete."
        ;;
    
    shell)
        log_info "Opening shell in API container..."
        $COMPOSE_CMD -f $COMPOSE_FILE -p $PROJECT_NAME exec api /bin/bash
        ;;
    
    psql)
        log_info "Connecting to PostgreSQL..."
        $COMPOSE_CMD -f $COMPOSE_FILE -p $PROJECT_NAME exec postgres psql -U apex -d apex_db
        ;;
    
    status)
        $COMPOSE_CMD -f $COMPOSE_FILE -p $PROJECT_NAME ps
        ;;
    
    health)
        log_info "Checking service health..."
        curl -s http://localhost:8080/api/health | jq .
        ;;
    
    *)
        echo "Usage: $0 {up|build|down|clean|logs|migrate|shell|psql|status|health}"
        echo ""
        echo "Commands:"
        echo "  up      - Start all services (default)"
        echo "  build   - Build and start services"
        echo "  down    - Stop services"
        echo "  clean   - Stop services and remove volumes"
        echo "  logs    - Follow logs (optional: service name)"
        echo "  migrate - Run database migrations"
        echo "  shell   - Open shell in API container"
        echo "  psql    - Connect to PostgreSQL"
        echo "  status  - Show service status"
        echo "  health  - Check API health"
        exit 1
        ;;
esac
