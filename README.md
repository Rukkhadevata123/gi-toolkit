## Genshin Impact Toolkit using MinHook-rs

### Features

![](./demo.png)

### Usage

- Download the zip package from the Release page. After extracting, you will get the `assets` folder and `gi-toolkit.exe`.
- If you do not need to hook the Bilibili Login Panel, simply run `gi-toolkit.exe` as administrator. Make sure the program's directory and the `assets` folder are in the same location (for example, if you run `..\gi-toolkit.exe`, then `gi-toolkit.exe` should be in the parent directory of `assets`).
- If you need the Bilibili Login Panel hook feature, place the `assets` folder into the game directory (where `YuanShen.exe` is located, e.g., `D:\Program Files\Genshin Impact\Genshin Impact Game`), and put `gi-toolkit.exe` in the parent directory. Then, create a batch script to launch the program (e.g., launch.bat):

  ```cmd
  @echo off
  cd "D:\Program Files\Genshin Impact\Genshin Impact Game"
  sudo ..\gi-toolkit.exe
  ```

- For details on using the Bilibili login feature, please refer to the "usage" button in the program.
- Example `login.json` (single line):

  ```json
  {"code":0,"data":{"access_key":"qwertyuiop_t1","game_id":"4963","uid":2147483647,"uname":"A Cool Name"}}
  ```
