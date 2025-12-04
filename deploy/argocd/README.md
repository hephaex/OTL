# ArgoCD Deployment for OTL

## Prerequisites

1. ArgoCD installed in the cluster
2. `kubectl` configured with cluster access
3. ArgoCD CLI installed (optional)

## Installation

### 1. Install ArgoCD (if not already installed)

```bash
kubectl create namespace argocd
kubectl apply -n argocd -f https://raw.githubusercontent.com/argoproj/argo-cd/stable/manifests/install.yaml
```

### 2. Create the OTL Project

```bash
kubectl apply -f deploy/argocd/project.yaml
```

### 3. Create the OTL Application

```bash
kubectl apply -f deploy/argocd/application.yaml
```

### 4. Access ArgoCD UI

```bash
# Get the initial admin password
kubectl -n argocd get secret argocd-initial-admin-secret -o jsonpath="{.data.password}" | base64 -d

# Port forward to access UI
kubectl port-forward svc/argocd-server -n argocd 8080:443

# Open https://localhost:8080 in browser
```

## Using ArgoCD CLI

```bash
# Login
argocd login localhost:8080

# List applications
argocd app list

# Sync application
argocd app sync otl

# Get application status
argocd app get otl

# Rollback
argocd app rollback otl <revision>
```

## Environments

- **Production**: `main` branch → `otl` namespace
- **Staging**: `develop` branch → `otl-staging` namespace (configure separately)

## Monitoring

ArgoCD provides built-in monitoring:
- Sync status
- Health status
- Resource tree visualization
- Git commit tracking

Author: hephaex@gmail.com
