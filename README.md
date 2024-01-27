# fap
A terminal based file explorer that uses vim motions to avoid using a mouse.  Can be used as a replacement for cd alongside adding a bash script.

If your terminal is bash, in your .bashrc file add:

```
fap() {
    local result=$(*insert absolute path to file here* "$@")
    [ -n "$result" ] && cd -- "$result"
}
```

This will allow you to type fap in your terminal, (so long as you replace the appropriate text with the file path to the downloaded file) 
opening fap and allowing you to change your cd to the cd you went to in the fap instance.

Space exits out of fap and sets your cd to the cd listed in fap \
ESC exits out of fap, returning the original cd you started from \
Enter either goes into the directory or opens the selected file \
hjkl move the cursor: \
  &emsp; h = left \
  &emsp; j = down \
  &emsp; k = up \
  &emsp; l = right \
H moves the cursor to the top of the screen \
M moves the cursor to the middle of the screen \
L moves the cursor to the bottom of the screen \
gg moves the cursor to the first line in the buffer \
G moves the cursor to the last line in the buffer \
CTRL + e scrolls down by one line (doesn't move cursor) \
CTRL + y scrolls up by one line (doesn't move cursor) \
CTRL + f scrolls down by one page (cursor goes to first line) \
CTRL + b scrolls up by one page (cursor goes to bottom line) \
CTRL + d scrolls down by half a page \
CTRL + u scrolls up by half a page

Like normal VIM motions, you can type a number before the motion, and it'll run that motion that many times.
For example, type 10j, this will move the cursor down 10 times.
