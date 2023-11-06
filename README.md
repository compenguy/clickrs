# clickrs
An keyboard/mouse clicker written in rust.  The uinput default feature should
work just fine for any linux system with a kernel supporting uinput, including
wayland desktop. For X11 systems, the x11 feature may provide more features.

# Build requirements
The x11 feature uses the libxtst library, so libxtst-dev (or whatever package
provides its headers) is required during build.

# Runtime requirements
The uinput feature, in order to be able to read numlock state, requires that 
the user running the application to have +rw permissions to the
`/dev/input/event*` device for the keyboard that should be checked for the
active state toggle (numlock).  This typically means being added as a member of
the "input" system group.
