If your terminal is bash, in your .bashrc file add:

```
fap() {
    local result=$(/home/river/Desktop/Projects/File_Access_Pathfinder/fap/target/release/fap "$@")
    [ -n "$result" ] && cd -- "$result"
}
```

This will allow you to type fap in your terminal, opening fap and allowing you to change your cd to the cd you went to in the fap instance.

Space exits out of fap and sets your cd to the cd listed in fap
ESC exits out of fap, returning the original cd you started from
Enter either goes into the directory or opens the selected file
hjkl move the cursor:
  h = left
  j = down
  k = up
  l = right
H moves the cursor to the top of the screen
M moves the cursor to the middle of the screen
L moves the cursor to the bottom of the screen
gg moves the cursor to the first line in the buffer
G moves the cursor to the last line in the buffer

Like normal VIM motions, you can type a number before the motion, and it'll run that motion that many times.
For example, type 10j, this will move the cursor down 10 times.
