import { useCallback, useEffect, useState } from "react";
import type { ReactNode } from "react";
import { Navigate, NavLink, Route, Routes } from "react-router";
import { cn } from "cn";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { fetchMe, signOut } from "./auth";
import type { User } from "./auth";
import { Landing } from "./Landing";
import { MyExercisesScreen } from "./MyExercisesScreen";
import { WeighInsScreen } from "./WeighInsScreen";

/** The landing page for visitors, the app screens for a signed-in user. */
export function App() {
  // undefined: not known yet; null: nobody is signed in.
  const [user, setUser] = useState<User | null | undefined>(undefined);
  const handleUnauthorized = useCallback(() => setUser(null), []);

  useEffect(() => {
    fetchMe().then(setUser, () => setUser(null));
  }, []);

  async function handleSignOut() {
    await signOut();
    setUser(null);
  }

  return (
    <div className="mx-auto max-w-[720px] px-5 pb-[env(safe-area-inset-bottom)]">
      <header className="flex min-h-17 items-center justify-between gap-3 py-5">
        <span className="text-lg font-bold tracking-wide text-primary">Bodyweight</span>
        {user && (
          <div className="flex min-w-0 items-center gap-2 text-sm">
            <Avatar className="size-7">
              {user.avatar_url && <AvatarImage src={user.avatar_url} alt="" />}
              <AvatarFallback className="bg-card font-semibold text-primary" aria-hidden="true">
                {user.login.charAt(0).toUpperCase()}
              </AvatarFallback>
            </Avatar>
            <span className="truncate" title={user.display_name}>
              {user.login}
            </span>
            <Button
              variant="link"
              className="px-0 pl-2 text-muted-foreground underline"
              onClick={handleSignOut}
            >
              Log out
            </Button>
          </div>
        )}
      </header>

      {user === null && <Landing onSignIn={setUser} />}
      {user && <Screens onUnauthorized={handleUnauthorized} />}

      <footer className="pt-6 pb-8 text-sm text-muted-foreground">bodyweight.ru</footer>
    </div>
  );
}

/** The screens of a signed-in user. The other screens of docs/spec.md §4.1 come later. */
function Screens({ onUnauthorized }: { onUnauthorized: () => void }) {
  return (
    <>
      <nav className="flex gap-5 border-b">
        <ScreenLink to="/weigh-ins">Weigh-ins</ScreenLink>
        <ScreenLink to="/exercises">My exercises</ScreenLink>
      </nav>
      <main>
        <Routes>
          <Route path="/weigh-ins" element={<WeighInsScreen onUnauthorized={onUnauthorized} />} />
          <Route
            path="/exercises"
            element={<MyExercisesScreen onUnauthorized={onUnauthorized} />}
          />
          <Route path="*" element={<Navigate to="/weigh-ins" replace />} />
        </Routes>
      </main>
    </>
  );
}

function ScreenLink({ to, children }: { to: string; children: ReactNode }) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        cn(
          "-mb-px border-b-2 py-2.5",
          isActive ? "border-primary text-foreground" : "border-transparent text-muted-foreground",
        )
      }
    >
      {children}
    </NavLink>
  );
}
