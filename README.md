# Timeline

A basic library for building timelines in Rust.  This library is designed to be
used for tracing applications that want to show different operations on a
timescale.  The general usage is to show when tasks go onto which CPU, which
tasks wake which other task up, and on what CPU that task ends up running.  This
is generic enough however to map anything that is time based and has a start and
end time.
