# Akiv, the hyper minimalistic daily planner

Akiv helps you plan and keep track of your daily tasks. When presented
with a choice between usefulness and simplicity, we choose
simplicity. Akiv is so simple its usefulness is doubtful.

## Usage

Akiv works on an ordered list of tasks for the day. All operations are
applied to the list of tasks *for the current day*. At the end of
everyday day (on your local time zone), a new lists of tasks begins.

Basically, using akiv means:

 - Adding new tasks.
 - Moving to the current task to the next.
 - Consulting the list of tasks.

### Adding tasks

```sh
akiv add "Finish writing the README" "20 minutes"
```

Adds a new task at the end of the list. It takes two mandatory
parameters: a description, and an estimated time to complete (see
[valid duration
strings](https://www.freedesktop.org/software/systemd/man/systemd.time.html#Parsing%20Time%20Spans)).


You can also add a task at a given position, using the ```-a``` parameter.

### Listing tasks

```sh
akiv list
```

Prints the current list of tasks


