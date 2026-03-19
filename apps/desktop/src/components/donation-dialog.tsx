import { useEffect, useState } from "react";
import { Heart } from "lucide-react";
import { open as openUrl } from "@tauri-apps/plugin-shell";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";

const DONATE_URL = "https://github.com/sponsors/brilyyy";
const STORAGE_DONATED = "omah-donated";
const STORAGE_SNOOZED = "omah-donation-snoozed";
const SNOOZE_MS = 3 * 24 * 60 * 60 * 1000; // 7 days

function shouldShow(): boolean {
  if (localStorage.getItem(STORAGE_DONATED)) return false;
  const snoozed = localStorage.getItem(STORAGE_SNOOZED);
  if (snoozed && Date.now() - Number(snoozed) < SNOOZE_MS) return false;
  return true;
}

export function DonationDialog() {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    // Small delay so the app loads before interrupting the user
    const id = setTimeout(() => setOpen(shouldShow()), 800);
    return () => clearTimeout(id);
  }, []);

  function handleDonate() {
    openUrl(DONATE_URL);
    // Give a moment so the link opens, then mark donated
    setTimeout(() => {
      localStorage.setItem(STORAGE_DONATED, "true");
      setOpen(false);
    }, 300);
  }

  function handleDonated() {
    localStorage.setItem(STORAGE_DONATED, "true");
    setOpen(false);
  }

  function handleLater() {
    localStorage.setItem(STORAGE_SNOOZED, String(Date.now()));
    setOpen(false);
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(v) => {
        if (!v) handleLater();
      }}
    >
      <DialogContent className="sm:max-w-sm">
        <DialogHeader>
          <div className="mb-2 flex justify-center">
            <span className="flex size-12 items-center justify-center rounded-full bg-primary/10">
              <Heart className="size-6 text-primary" fill="currentColor" />
            </span>
          </div>
          <DialogTitle className="text-center">Enjoying omah?</DialogTitle>
          <DialogDescription className="text-center leading-relaxed">
            omah is free and open-source. If it saves you time, consider
            supporting continued development with a small donation.
          </DialogDescription>
        </DialogHeader>

        <DialogFooter className="flex-col gap-2 sm:flex-col">
          <Button className="w-full gap-2" onClick={handleDonate}>
            <Heart className="size-4" fill="currentColor" />
            Support omah
          </Button>
          <div className="flex gap-2">
            <Button
              variant="outline"
              className="flex-1 text-muted-foreground"
              onClick={handleDonated}
            >
              I've donated ♥
            </Button>
            <Button
              variant="ghost"
              className="flex-1 text-muted-foreground"
              onClick={handleLater}
            >
              Maybe later
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
