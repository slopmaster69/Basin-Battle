#!/bin/bash
# Double-click in Finder to set up and build everything on this Mac.
cd "$(dirname "$0")" || exit 1
bash scripts/setup-mac.sh
status=$?
echo
if [ $status -eq 0 ]; then echo "Done. You can close this window."; else echo "Setup stopped (exit $status). Scroll up for the reason."; fi
read -r -p "Press Return to close." _
