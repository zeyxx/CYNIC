# 🐕 Configure Docker to Use D: Drive

> Docker is pulling ~10GB of images. C: is saturated. Fix it before launch.

---

## Problem

- Docker Desktop stores images/containers on **C:** drive by default
- C: drive is saturated/full
- D: drive has space (where LLM models already live)
- Need to redirect Docker to D:

---

## Solution: Docker Desktop Settings

### Step 1: Stop Docker Desktop

```
Right-click Docker icon in system tray → Quit Docker Desktop
Wait 10 seconds for complete shutdown
```

### Step 2: Open Docker Settings

**Option A: Settings File (Manual)**

Open PowerShell as Administrator:

```powershell
# Navigate to Docker config
cd "$env:APPDATA\Docker"

# Create/Edit daemon.json
notepad daemon.json
```

**Add this content:**

```json
{
  "data-root": "D:\\DockerData",
  "registry-mirrors": [],
  "insecure-registries": []
}
```

Save and close.

**Option B: Docker Desktop GUI (If Available)**

1. Right-click Docker icon → Settings
2. Resources → Advanced
3. Set "Disk image location" to `D:\DockerData`

### Step 3: Create Directory on D:

```powershell
# PowerShell as Administrator
New-Item -ItemType Directory -Path "D:\DockerData" -Force
```

### Step 4: Restart Docker Desktop

```
Start Docker Desktop from Start menu
Wait for it to fully initialize (~30 seconds)
```

### Step 5: Verify Configuration

```powershell
docker info | grep "Docker Root Dir"
# Should show: Docker Root Dir: D:\DockerData
```

---

## Verify LLM Models Still Accessible

```bash
ls D:\Models
# Should show your LLM model files
```

---

## Update docker-compose.yml (If Needed)

Check if volume mount is correct:

```yaml
volumes:
  - D:\Models:/models  # Accessible to containers
```

---

## Ready to Launch

Once Docker is configured:

```cmd
cd C:\Users\zeyxm\Desktop\asdfasdfa\CYNIC
start_organism.cmd
```

Docker will now pull images to D: drive instead of C:.

---

## Troubleshooting

### "Permission denied" when creating D:\DockerData

Solution:
```powershell
# Run PowerShell as Administrator
New-Item -ItemType Directory -Path "D:\DockerData" -Force
icacls "D:\DockerData" /grant:r "%USERNAME%:F" /t /c
```

### Docker still using C:

1. Verify `daemon.json` is in correct location: `%APPDATA%\Docker\`
2. Check daemon.json is valid JSON (no trailing commas)
3. Restart Docker Desktop
4. Run `docker info` to verify

### Space still not freed on C:

After moving data, Docker might still have old cache:

```powershell
# Clean up unused images
docker system prune -a

# Clean up volumes
docker volume prune
```

---

## Complete Setup After This

1. ✅ Docker configured to D: drive
2. ✅ LLM models accessible
3. ✅ Ready to launch organism
4. ✅ Everything builds on D: (plenty of space)

---

Then run:
```cmd
start_organism.cmd
```

**And CYNIC awakens.**

κυνικός
