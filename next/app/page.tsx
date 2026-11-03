import ThemeToggle from "./theme-toggle";
const repo = "https://github.com/wrestle-R/Internet-swap";
export default function Home() {
  return (
    <main>
      <nav aria-label="Main navigation">
        <a className="brand" href="/" aria-label="Internet Swap home">
          <span className="mark" aria-hidden="true">
            ⇄
          </span>{" "}
          Internet Swap
        </a>
        <div className="nav-actions">
          <ThemeToggle />
          <a href={repo}>
            Source code <span aria-hidden="true">↗</span>
          </a>
        </div>
      </nav>
      <section className="hero">
        <div className="intro">
          <p className="eyebrow">
            <span /> A SMALL WI-FI COMPANION FOR LINUX
          </p>
          <h1>
            A little less
            <br />
            babysitting
            <br />
            your <em>Wi-Fi.</em>
          </h1>
          <p className="lede">
            I leave things running and go AFK, and sometimes the Wi-Fi decides
            it&apos;s done for the day. So I&apos;m building something to keep
            an eye on it.
          </p>
          <div className="actions">
            <a className="primary" href={`${repo}/releases/tag/v0.1.0`}>
              Get Internet Swap <span aria-hidden="true">↓</span>
            </a>
            <a href="#how-it-works">
              How it works <span aria-hidden="true">↘</span>
            </a>
          </div>
          <p className="footnote">
            Linux · NetworkManager · Free & open source
          </p>
        </div>
        <div className="preview-wrap">
          <div className="preview-caption">
            <span>01 / THE LITTLE WINDOW</span>
            <span>INTERFACE PREVIEW</span>
          </div>
          <div
            className="preview"
            aria-label="Illustration of the desktop interface, with example network names"
          >
            <div className="window-chrome">
              <i />
              <i />
              <i />
              <span>Internet Swap</span>
            </div>
            <div className="preview-inner">
              <div className="preview-title">
                <span className="mark">⇄</span>
                <div>
                  Internet Swap<small>Your Wi-Fi, with a little backup.</small>
                </div>
              </div>
              <div className="auto">
                <div>
                  Automatic switching
                  <small>On, even when the window is closed.</small>
                </div>
                <span className="toggle" aria-hidden="true" />
              </div>
              <p className="status">
                <b /> WATCHING
              </p>
              <h2>Saved Wi-Fi</h2>
              <p className="muted">Watching your Wi-Fi.</p>
              <div className="live">
                <div>
                  <small>↓ Live download</small>
                  <strong>
                    — <span>MB/s</span>
                  </strong>
                </div>
                <div>
                  <small>↑ Live upload</small>
                  <strong>
                    — <span>KB/s</span>
                  </strong>
                </div>
              </div>
              <div className="list-heading">
                Saved & available <span>Test networks ↗</span>
              </div>
              <div className="network">
                <div>
                  Saved Wi-Fi <span className="pill">Connected</span>
                </div>
                <small>2.4 GHz · example network</small>
                <p>Not speed-tested yet</p>
              </div>
              <div className="network">
                <div>
                  Another saved Wi-Fi <span className="connect">Connect</span>
                </div>
                <small>5 GHz · example network</small>
                <p>Not speed-tested yet</p>
              </div>
              <p className="preview-limit">
                One adapter, one connection. Switching can interrupt active
                requests.
              </p>
            </div>
          </div>
          <p className="caption">
            Example networks shown. Your app uses real local status.
          </p>
        </div>
      </section>
      <section id="how-it-works" className="how">
        <div className="section-label">LESS FUSS, A FEW GROUND RULES.</div>
        <div className="rules">
          <article>
            <span className="number">01</span>
            <div>
              <h2>Watch what&apos;s available.</h2>
              <p>
                See saved, secured Wi-Fi networks nearby, signal strength and
                live traffic. Passwords stay with NetworkManager.
              </p>
            </div>
          </article>
          <article>
            <span className="number">02</span>
            <div>
              <h2>Switch with a reason.</h2>
              <p>
                Recent test results can identify a better connection. Repeated
                internet check failures trigger recovery, with a cooldown to
                avoid constant switching.
              </p>
            </div>
          </article>
          <article>
            <span className="number">03</span>
            <div>
              <h2>One switch to switch it off.</h2>
              <p>
                Monitoring runs in the background. Turn automatic switching off
                whenever you want. The desktop window and Hyprland sidebar share
                the same control.
              </p>
            </div>
          </article>
        </div>
      </section>
      <section className="story">
        <span className="section-label">WHY I MADE THIS</span>
        <div>
          <h2>
            Same laptop.
            <br />
            Different Wi-Fi results.
          </h2>
          <p>
            I recently found that the 2.4 GHz and 5 GHz networks I use gave me
            different speeds. In a few short tests, 2.4 GHz had slightly better
            download and 5 GHz had better upload. That got me thinking, why am I
            checking and switching this myself?
          </p>
          <p>
            Those were results from my setup, not a rule about Wi-Fi bands.
            Internet Swap uses measurements from your connections.
          </p>
        </div>
      </section>
      <aside className="reality">
        <span aria-hidden="true">↳</span>
        <div>
          <h2>A quick reality check.</h2>
          <p>
            One Wi-Fi adapter generally joins one network at a time. Testing
            another connection or switching can interrupt downloads, calls and
            chat requests. This app doesn&apos;t combine bandwidth or guarantee
            uninterrupted sessions. Speed tests use internet data; USB tethering
            is outside its selection logic.
          </p>
        </div>
      </aside>
      <footer>
        <a className="brand" href="/">
          Internet Swap{" "}
          <span className="footer-mark" aria-hidden="true">
            ⇄
          </span>
        </a>
        <span>
          Made by <a href="https://github.com/wrestle-R">Russel</a>, coz I
          needed it.
        </span>
        <a href={`${repo}/releases`}>Releases ↗</a>
        <a href={repo}>GitHub ↗</a>
      </footer>
    </main>
  );
}
