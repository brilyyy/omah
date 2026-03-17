import { cn } from "@/lib/utils";

export default function SplashScreen({ visible }: { visible: boolean }) {
  return (
    <div
      className={cn(
        "pointer-events-none fixed inset-0 z-50 flex flex-col items-center justify-center",
        "bg-background transition-opacity duration-[5000ms] ease-in-out",
        visible ? "opacity-100" : "opacity-0",
      )}
      aria-hidden={!visible}
    >
      <div className="flex flex-col items-center gap-5">
        <span className="text-4xl font-bold tracking-tighter text-foreground select-none">
          omah
        </span>
        <div className="flex gap-1.5">
          {[0, 1, 2].map((i) => (
            <span
              key={i}
              className="size-1.5 rounded-full bg-muted-foreground/40 animate-pulse"
              style={{ animationDelay: `${i * 200}ms` }}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
