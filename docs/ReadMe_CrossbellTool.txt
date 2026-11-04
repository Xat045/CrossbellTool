CrossbellTool 2.5: decompiles and builds the files of Trails from Zero and Trails to Azure (PC)
  The Legend of Heroes: Trails from Zero   (zero.exe)
  The Legend of Heroes: Trails to Azure    (ao.exe)

WHAT IT DOES
  Unpacks files from the data folder of the game or a translation mod into ordinary files for translation (dialogues .json,
  tables .xlsx, images .png) and builds your edits back into game files (.bin, ._dt, .itp) with the same
  folder structure. Without edits, files build byte-for-byte identical to the original.
  Translation comparison is a separate program, CrossbellCompare.exe.
  Requires a graphics card with OpenGL 2 or later.

──────────────────────────────── QUICK START

  1. 'Game folder or files': the game folder or a translation mod's folder; its data folder is used. You can
     drop the folder on the window.
  2. The 'What to decompile' window opens: tick what you need and click 'Done'.
  3. 'Project folder (for editing)' is where the files go; 'Output folder' is the folder for the result.
  4. 'Decompile all' → edit the files in the project folder.
  5. 'Build' → tick the folders in the 'What to build' window → 'Build'.
     Copy the result into the game's data folder, replacing the files (keep the originals).

──────────────────────────────── GAME

  Detected automatically: zero.exe / ao.exe, or "Zero" / "Azure" in the folder name. The detected game is shown on the right
  above 'Game folder or files'; you can also choose it manually in the 'Game' list. Each game has its own project
  and output folders; on the first run the fields are empty, so choose the folders yourself.

──────────────────────────────── DECOMPILING

  The 'What to decompile' window (opens by itself after you choose a folder; reopen it with the 'Files: N of M' button):
  • the data folders under 'Text for translation' and 'Textures': how many scenes, tables and images,
    English (._us) or Japanese files, and the size;
  • everything is ticked by default; quick choices: 'Text only', 'Text+textures', 'All', 'Clear all';
  • at the bottom: how much the unpacked files will take and roughly how long (the time appears after the first
    decompile longer than 30 s on this computer).
  Disk space is checked before decompiling, and the program warns you if it's short.
  'File mask' limits the work to specific files: c0100, t_town, c01. (comma-separated; empty means all).
  Files you have already edited are not overwritten when you decompile again.
  Old-format textures (with a palette: models, maps, monsters) are skipped: they have no text.

  In the project folder you will find:
    scena_us\..json   dialogues: translate the strings in "text"
                      (\n is a new line in the window, {wait} waits for a key press; for codes, see 'Service codes' in the CrossbellCompare guide);
    text_us\..xlsx    tables (items, towns, quests, books): edit column B;
    ..png             images (save with transparency, don't change the size);
    ..dds             images, if the .dds format was chosen when decompiling: the same BC7 blocks as in .itp, without
                      re-encoding; when building they move into .itp as they are (those saved in another
                      DDS format are recompressed, like .png). Edit them in an editor with DDS support.

──────────────────────────────── BUILDING

  The 'What to build' window: the project folders and how many files changed in each; at the bottom, roughly how much
  will be written and how long it will take (the time appears after the first build longer than 10 s). Only changed
  files of the ticked folders are built, back into .bin, ._dt and .itp, into the 'Output folder' with the same
  structure (scena_us, text_us…).
  Reliability:
  • disk space is checked before building;
  • if you choose the game's data folder or the game folder, the program asks for confirmation;
  • a file is written through a temporary one: if writing fails, the previous file (and the game's file) stays intact;
  • after writing, each file is reread, and scenes and tables are parsed back: a file the game
    couldn't read shows up in the log at once ("✖") and counts in the errors total;
  • tables: if a service name (a file or model name with "_", for example chr0000_c15) was translated in Excel by
    accident, it goes back to the game's original when building, with a warning in the log.
    Numbers are not touched, because they may be changed on purpose.

──────────────────────────────── WINDOW

  • At the bottom are the 'Help' and 'Log' tabs; 'Open project', 'Open output', 'Stop'.
  • A long decompile or build (over 10 s) ends with a Windows notification and a sound,
    and the program's taskbar button flashes.
  • Light and dark theme: the button at the bottom right. The RU / EN button next to it switches the language.

──────────────────────────────── FILES NEXT TO THE PROGRAM

  CrossbellTool.ini: settings (folders, mask, decompile and build speed); CrossbellTool.log: the log.

──────────────────────────────── COMMAND LINE

  CrossbellTool decompile --game zero|azure --data <data folder> --proj <project folder>
                       [--types scena,tables,textures] [--mask ...] [--overwrite]
  CrossbellTool compile   --proj <project folder> --out <output folder> [--data <data folder>] [--mask ...] [--all]
                       (--data compares tables with the original; --all builds everything, not only what changed)
  CrossbellTool file [--game zero|azure] <file>: one file: .bin/._dt/.itp → .json/.xlsx/.png and back
