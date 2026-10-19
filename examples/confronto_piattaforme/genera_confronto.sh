#!/usr/bin/env bash
# genera_confronto.sh — (ri)genera confronto.html confrontando gli screenshot
# di ogni esempio fra Linux, Windows e macOS.
#
# Uso:
#   ./genera_confronto.sh        genera/regera confronto.html
#
# Sorgenti attese (cartelle sorelle, relative a questo script):
#   ../immagini_linux/<esempio>.png
#   ../immagini_windows/<esempio>.png      (fallback: esempi/immaginiwin)
#   ../immagini_macos/<esempio>.png
# Gli esempi privi di screenshot mostrano un placeholder "non disponibile".

set -euo pipefail

BASE="$(cd "$(dirname "$0")" && pwd)"
EX="$BASE/.."                                  # cartella examples/
OUT="$BASE/confronto.html"

DIR_LIN="$EX/immagini_linux"
DIR_WIN="$EX/immagini_windows"
DIR_MAC="$EX/immagini_macos"
NOTE_WIN=""

# Le screenshot Windows reali sono in <repo>/esempi/immaginiwin, fuori da
# examples/: se la cartella sorella è vuota usiamo quella, sennò il
# confronto resterebbe tutto placeholder dalla parte Windows.
# La radice del repo la risaliamo guardando dove sta esempi/immaginiwin.
if [ ! -d "$DIR_WIN" ] || [ -z "$(find "$DIR_WIN" -maxdepth 1 -name '*.png' -print -quit 2>/dev/null)" ]; then
    probe="$BASE"
    for _ in 1 2 3 4 5 6; do
        probe="$(dirname "$probe")"
        if [ -d "$probe/esempi/immaginiwin" ]; then
            DIR_WIN="$probe/esempi/immaginiwin"
            NOTE_WIN="sorgente Windows: esempi/immaginiwin"
            break
        fi
    done
fi

# --- raccolta nomi esempio (unione dei .png delle tre cartelle) ------------
nomi="$(
  for d in "$DIR_LIN" "$DIR_WIN" "$DIR_MAC"; do
    [ -d "$d" ] || continue
    find "$d" -maxdepth 1 -name '*.png' -printf '%f\n'
  done | sed 's/\.png$//' | sort -u
)"

tot=$(printf '%s\n' "$nomi" | sed '/^$/d' | wc -l)
n_lin=$( [ -d "$DIR_LIN" ] && find "$DIR_LIN" -maxdepth 1 -name '*.png' | wc -l || echo 0 )
n_win=$( [ -d "$DIR_WIN" ] && find "$DIR_WIN" -maxdepth 1 -name '*.png' | wc -l || echo 0 )
n_mac=$( [ -d "$DIR_MAC" ] && find "$DIR_MAC" -maxdepth 1 -name '*.png' | wc -l || echo 0 )

NOTE_HTML=""
if [ -n "$NOTE_WIN" ]; then
    NOTE_HTML=" &middot; $NOTE_WIN"
fi

# --- cella di una piattaforma: immagine oppure placeholder ------------------
# Il src è sempre relativo al file HTML generato, anche quando la cartella
# sorgente è fuori da examples/ (realpath --relative-to).
cella() {  # $1=cartella $2=nome esempio $3=etichetta $4=classe colore
  local dir="$1" nome="$2" plat="$3" cls="$4" src
  if [ -f "$dir/$nome.png" ]; then
    src="$(realpath --relative-to="$BASE" "$dir/$nome.png")"
    printf '        <figure>\n'
    printf '          <span class="etichetta %s">%s</span>\n' "$cls" "$plat"
    printf '          <a href="%s" target="_blank"><img src="%s" alt="%s — %s" loading="lazy"></a>\n' \
      "$src" "$src" "$nome" "$plat"
    printf '        </figure>\n'
  else
    printf '        <figure class="mancante">\n'
    printf '          <span class="etichetta %s">%s</span>\n' "$cls" "$plat"
    printf '          <div class="placeholder">screenshot non disponibile</div>\n'
    printf '        </figure>\n'
  fi
}

# --- generazione HTML -------------------------------------------------------
{
  cat <<HTML
<!DOCTYPE html>
<html lang="it">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>ru_wx — confronto piattaforme</title>
<style>
  :root {
    --bg: #14171c; --panel: #1d222a; --testo: #e6e9ee; --tenue: #9aa4b2;
    --lin: #e9a23b; --win: #4fa3f7; --mac: #b48ef7;
  }
  * { box-sizing: border-box; }
  body { margin: 0; padding: 2rem; background: var(--bg); color: var(--testo);
         font: 15px/1.5 system-ui, -apple-system, "Segoe UI", sans-serif; }
  h1 { margin: 0 0 .3rem; font-size: 1.6rem; }
  .sommario { color: var(--tenue); margin-bottom: 1.5rem; }
  .esempio { background: var(--panel); border: 1px solid #2a313c; border-radius: 10px;
             padding: 1rem 1.2rem 1.2rem; margin-bottom: 1.4rem; }
  .esempio h2 { margin: 0 0 .9rem; font-size: 1.1rem; font-family: ui-monospace, monospace;
                color: #fff; border-bottom: 1px solid #2a313c; padding-bottom: .55rem; }
  .riga { display: grid; grid-template-columns: repeat(3, 1fr); gap: 1rem; }
  figure { margin: 0; min-width: 0; }
  .etichetta { display: inline-block; font-size: .75rem; font-weight: 700; letter-spacing: .06em;
               text-transform: uppercase; padding: .15rem .55rem; border-radius: 999px;
               margin-bottom: .45rem; color: #14171c; }
  .etichetta.lin { background: var(--lin); }
  .etichetta.win { background: var(--win); }
  .etichetta.mac { background: var(--mac); }
  figure img { width: 100%; height: auto; border: 1px solid #2a313c; border-radius: 6px;
               background: #000; display: block; }
  .placeholder { display: flex; align-items: center; justify-content: center; aspect-ratio: 16/10;
                 border: 1px dashed #39414e; border-radius: 6px; color: var(--tenue);
                 font-size: .85rem; text-align: center; padding: .5rem; }
  @media (max-width: 900px) { .riga { grid-template-columns: 1fr; } }
</style>
</head>
<body>
<h1>ru_wx — confronto piattaforme</h1>
<p class="sommario">
  $tot esempi &middot; Linux: $n_lin screenshot &middot; Windows: $n_win screenshot &middot;
  macOS: $n_mac screenshot &middot; generato il $(date '+%Y-%m-%d %H:%M')
  (ricreare con <code>./genera_confronto.sh</code>)$NOTE_HTML
</p>
HTML

  # ogni esempio = una riga: nome sopra, sotto le tre piattaforme affiancate
  while IFS= read -r nome; do
    [ -n "$nome" ] || continue
    printf '<section class="esempio">\n'
    printf '  <h2>%s</h2>\n' "$nome"
    printf '  <div class="riga">\n'
    cella "$DIR_LIN" "$nome" "Linux"   "lin"
    cella "$DIR_WIN" "$nome" "Windows" "win"
    cella "$DIR_MAC" "$nome" "macOS"   "mac"
    printf '  </div>\n'
    printf '</section>\n'
  done <<EOF
$nomi
EOF

  cat <<'HTML'
</body>
</html>
HTML
} > "$OUT"

echo "scritto: $OUT"
echo "esempi: $tot | linux: $n_lin | windows: $n_win | macos: $n_mac"
