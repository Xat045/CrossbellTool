CrossbellCompare 2.2: translation compare for Trails from Zero / Trails to Azure

WHAT IT DOES
  Shows the text of two languages side by side (Japanese, English or Russian, in any pair; usually the original
  on the left and the translation on the right), line by line, with search across both columns, a dictionary
  and character portraits. The 'Checks' tab shows translation errors: lost codes and special signs,
  untranslated lines, placeholders. CrossbellCompare only reads translation files and writes nothing to them.
  Requires a graphics card with OpenGL 2 or later.

──────────────────────────────── QUICK START

  Which folders to choose:
  • the game folder or its data: .bin and ._dt files are read directly, with no need to decompile;
  • a translation mod's folder with data and data_backup: choose it in both fields, and Russian comes
    from data and English from data_backup;
  • CrossbellTool projects (.json, .xlsx);
  • Japanese text: a folder with only Japanese files (for example a copy of the game's data\scena): if
    a folder has both scena and scena_us, English is used. The Russian Zero/Azure was translated from English,
    so the codes in 'Checks' won't match Japanese; use English for checking.
  For example: the Trails from Zero game folder on the left,
  and a CrossbellTool project or an unpacked translation mod on the right.
  Folders can be dropped on the window: the left half sets the left field, the right half the right one
  (two folders at once: the first on the left, the second on the right). Then 'Load', type a word, Enter.
  The game (Zero or Azure) is detected from the folder (zero.exe / ao.exe, or "Zero" / "Azure" in the name).

──────────────────────────────── LOADING

  • On start, CrossbellCompare loads the last folders and opens where you stopped:
    the same search word and line, the same tab, 'Checks' section, expanded group and scroll position.
  • The load cache is the CrossbellCompare_cache folder next to the program; Shift + 'Load' (or Shift+F5)
    reloads without the cache.
  • Files from different games (for example, English Zero and Russian Azure): lines are not shown,
    and a warning appears instead; if the program is wrong, use 'Show anyway'.
  • Table lines are matched by file structure: the pairs are correct even if a Russian table has its lines
    in a different order (as in Azure's t_quest). '- none -' means this language has no such line.

──────────────────────────────── SEARCH AND THE TABLE

  • Search across both columns: 'Whole words', 'Match case', 'Whole file'; the Cyrillic letters yo and ye count as one;
    dashes, quotes and ellipses count as the same; several words find lines that have all of them.
  • Blue is a search match, yellow is a dictionary word and its translation.
  • The speaker's portrait: a line with a face code (#0105F) shows the character's portrait on the right of the 'File' column.
  • The 'File' column shows the file name and the line's number (for .bin) or the file's line (for .json).
  • Text size: Ctrl+wheel or Ctrl+plus / minus, Ctrl+0 for normal (remembered).
  • Right-click: 'Copy the left column / right column / both / the file name', 'Open the left file',
    'Open the right file': .json opens in Notepad++ at this line, an .xlsx table in Excel
    or WPS on the 'Text' sheet with cell B of the line selected (an already open table, in that window).
  • Scrolling with the pressed wheel: press the wheel and move the mouse and the table scrolls by itself; press again to stop.
  • Ctrl+C copies the clicked column; double-clicking a cell opens a field for selecting
    part of the text; F11 is full screen. Signs that the game's font draws its own way are shown
    as in the game: ㈱ is ♥, ㍉ is è (Agnès). Hotkeys are at the end of this file.

──────────────────────────────── DICTIONARY

  Choose a dictionary with the 'Choose' button (.dat, .json or .txt: Bracer = translation). It highlights terms,
  counts the Russian variant next to 'Found', and marks lines 'as in the dictionary'.

──────────────────────────────── THE CHECKS TAB

  • 'Service codes': codes that don't match the original: {wait}, {color N}, {0x06}, {0x0D},
    {item[…]}, {}, #…F (face), #…V (voice), #…P, #…W, #…S, #…A, #…C, #…K, #…I, #…M
    (the number has no leading zeros: #0005F = #5F); lost or extra special signs
    (♥ ♪ ★ ☆ ◆ ━ ∴ ※ ← → ① ② ③ Ⅰ Ⅱ Ⅲ); broken text: a '\' in the middle of a line and code fragments
    (what remains of {x01}). What the codes mean: the 'Service codes' section at the end of this file.
  • 'Not translated': scene lines in English or Japanese (tables are not checked; identical ones
    on one line; debug scenes a0… are not checked).
  • 'Placeholders': %d, %s… not as many as in the original.
  • Identical errors are grouped: 'missing: {color 0}, 173 lines', the files and an example;
    'Show all' expands the group, 'Not an error · all' hides it entirely.
  • 'Open in Compare' (or a double click), 'Not an error' / 'Hidden' / 'Restore', 'Check again',
    scrolling with the pressed wheel. Right-click a finding for 'Open in Compare', 'Open the left /
    right file' at this line.
  • 'Export to Excel': one sheet per section; each finding has 5 lines before and after, as they are, with service
    codes (the finding in yellow).
  Some findings may look the same in the game (for example {color 0} at the very start of a line):
  remove those with the 'Not an error' button.

──────────────────────────────── ICONS INSTEAD OF CODES

  On screen only; nothing is written to files. The 'Icons: …' button in the settings row opens the window.
  • Images and portraits are in "data.dat" next to the program (encrypted; unpack the whole
    archive). Choose a game in the window: Zero or Azure (until chosen, by the folder of English files).
    Each game's set has: c_icon1 (#…I icons: sepith, items), t_item / t_item2 (the item icon
    in #…M, #060M is a brooch), t_quartz (the quartz element; Azure's master quartz is round), gamepad buttons
    (PlayStation, Xbox, Switch, Steam Deck, PS5), keyboard and mouse, t_magic (craft type S, C, CC, Sp,
    before the name and description), record (t_record achievement icons), character portraits.
  • An icon is also placed instead of {item[N]}.
  • Buttons (#160I and up) follow the game's "System → Options" screen, PlayStation by default:
      #165I △ (quick access, keyboard R) · #174I ← (detective notebook, A) · #175I → (battle notebook, D)
      #167I L1 (special slot, Azure, Q) · #176I (Zero) / #192I (Azure) OPTIONS (map, Tab)
      #194I □ (details, Azure, I) · #195I × (confirm, Azure, left mouse click)
    If you reassigned buttons in the game, assign yours in the window (Ctrl+click for a combination).
    'Device' sets whose icons are shown. No icon: a gray frame with the code (#18I).

──────────────────────────────── FILES NEXT TO THE PROGRAM

  KisekiTool_compare.ini: settings (folders, dictionary, theme, where you stopped, text size);
  KisekiCompare_buttons.txt: button assignments;
  CrossbellCompare_cache: the load cache, plus KisekiCompare_hidden.txt, the 'Not an error' list
  (Shift+'Load' doesn't delete it);
  data.dat: icons and portraits.

──────────────────────────────── HOTKEYS

Ctrl+F          go to the search field (from any tab)
Enter           search (in the search field)
F5              reload the folders
Shift+F5        reload without the cache (same as Shift + the 'Load' button)
Ctrl+1          the 'Compare' tab
Ctrl+2          the 'Checks' tab
Ctrl+C          copy the selected line's clicked column
Ctrl+wheel      larger / smaller text in the 'Compare' table (remembered)
Ctrl+plus/minus the same with keys; Ctrl+0 for normal size
F11             full screen; Esc leaves full screen
Esc             close a window or menu

Mouse
Double-click a finding in 'Checks'         open this line in 'Compare'
Right-click a line in 'Compare'            menu (copy, open the left / right file…)
Right-click a finding in 'Checks'          menu (open in Compare, open the left / right file)
Press the wheel in a table or list         the list scrolls where you move the mouse; press again to stop
Drag a folder onto the window              left half: the left field, right half: the right one

On the next start, CrossbellCompare opens where you stopped:
the same search word and line, the same tab, the same 'Checks' section and expanded group.

──────────────────────────────── SERVICE CODES

The main rule: every code in the English line must remain in the translated one, the same codes and as many of them.
Only the text between them is translated. CrossbellCompare compares the codes in each line and shows
what's missing or extra.

The notes below come from how the codes sit in the game's text and what is visible in CrossbellCompare (portraits, icons).
Where a purpose isn't confirmed, it says so. In brackets: how many times the code occurs
in the English text of both games.

──────────────────────────────── LINE BREAK AND THE WINDOW

\n          a new line in the dialogue window. In .json it is written exactly so: a backslash and the letter n.
            A lone '\' in the middle of a line breaks the text, and CrossbellCompare catches it.
{wait}      waits for a key press (137 939). It goes at the end of every line's window. If it's lost,
            the window closes by itself or the next text overlaps.
{0x06}, {0x0D} and other {0x..}  window control codes (1 343 / 551): they look like waiting and
            line breaks in special windows, exact purpose not confirmed. Leave them as they are.
{}          an empty code (107): leave it in the same place.

──────────────────────────────── COLOR

{color 5}   the text color from here on, the number is a color from the game's palette (14 550).
            {color 0} is the normal color. If {color 0} is lost, everything after the colored word
            stays colored.

──────────────────────────────── NAMES BY NUMBER

{item[501]} an item's name by number; the game inserts it itself (1 950). There is nothing to translate inside
            the brackets, and the number must not change.

──────────────────────────────── "#number + letter" CODES

The number may have leading zeros: #0005F and #5F are the same.

#…F   the speaker's face, a portrait from the face folder: #0105F = ka00105.png (67 323).
      CrossbellCompare shows this portrait next to the line.
#…V   voice, the voice file number (45 720). If it's lost, the line has no voice.
#…P   position of the window or portrait on screen, presumably (39 241)
#…W   text speed or a pause, presumably (2 245)
#…S   text size, presumably (1 139)
#…C   color, presumably, like {color} (1 115)
#…A   purpose not confirmed (784)
#…I   an icon in the text: sepith, a button, a symbol (653); CrossbellCompare draws it as an image
#…M   an item icon (332); CrossbellCompare draws it as an image
#…K   purpose not confirmed (180)
#…R   purpose not confirmed (20)
      Keep all # codes exactly as in the original and in the same place: usually at the start of a line,
      before the text.

──────────────────────────────── SPECIAL SIGNS

♥ ♪ ★ ━ ∴ ※ and the like are special images in the game's font. CrossbellCompare checks that there are
as many of them as in the original ('signs lost' / 'extra signs').
