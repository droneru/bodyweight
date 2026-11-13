import { useCallback, useEffect, useState } from "react";
import { Navigate, NavLink, Route, Routes } from "react-router";
import { fetchMe, signOut } from "./auth";
import type { User } from "./auth";
import { Landing } from "./Landing";
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
    <div className="page">
      <header className="top">
        <span className="logo">Bodyweight</span>
        {user && (
          <div className="account">
            <Avatar user={user} />
            <span className="login" title={user.display_name}>
              {user.login}
            </span>
            <button className="link" type="button" onClick={handleSignOut}>
              Log out
            </button>
          </div>
        )}
      </header>

      {user === null && <Landing onSignIn={setUser} />}
      {user && <Screens onUnauthorized={handleUnauthorized} />}

      <footer className="bottom">bodyweight.ru</footer>
    </div>
  );
}

/** The screens of a signed-in user. The other screens of docs/spec.md §4.1 come later. */
function Screens({ onUnauthorized }: { onUnauthorized: () => void }) {
  return (
    <>
      <nav className="tabs">
        <NavLink to="/weigh-ins">Weigh-ins</NavLink>
      </nav>
      <main>
        <Routes>
          <Route path="/weigh-ins" element={<WeighInsScreen onUnauthorized={onUnauthorized} />} />
          <Route path="*" element={<Navigate to="/weigh-ins" replace />} />
        </Routes>
      </main>
    </>
  );
}

function Avatar({ user }: { user: User }) {
  if (user.avatar_url) {
    return <img className="avatar" src={user.avatar_url} alt="" width={28} height={28} />;
  }
  return (
    <span className="avatar placeholder" aria-hidden="true">
      {user.login.charAt(0).toUpperCase()}
    </span>
  );
}
