# Akiv, the hyper minimalistic daily planner

Akiv helps you and keep a list of tasks to be done during the
day. When presented with a choice between usefulness and simplicity,
we choose simplicity.

Akiv is so simple its usefulness is doubtful.

## Usage

Akiv works on an ordered list of tasks for the day. Note that all
operations are applied to the list of tasks *for the current day*. At
the end of every day (taking into account your local time zone) a new
lists of tasks starts.

Using akiv generally means:

 - Adding new tasks.
 - Moving to the current task to the next.
 - Consulting the list of tasks.

### Adding tasks

```sh
akiv add "Finish writing the README" "20 minutes"

1. Finish writing the README (20m)
```

Adds a new task at the end of the list. It takes two mandatory
parameters: a description and an estimated time to complete (see
[valid duration
strings](https://www.freedesktop.org/software/systemd/man/systemd.time.html#Parsing%20Time%20Spans)).


You can also add a task at a given position, using the ```-a``` parameter.

### Listing tasks

```sh
akiv list
```

Prints the current list of tasks.

![First list](screenshots/list-1.png?raw=true)

The following fields are printed:

 * ```id``` the position of the task, which can be also used as an identifier for certain operations.
 * ```task``` the task's description.
 * ```started at``` the time at which the user started working on the task.
 * ```exp. duration``` the estimated time to complete the task.
 * ```ellapsed``` time spent working on the task (not counting pauses).
 * ```exp. end time``` the expected time at which the task es expected to be done.
 * ```pause time``` total duration of the pauses taken during this task.

### Start / Stop

At any time the user is either working or not working on her
tasks. ```akiv start``` and ```akiv stop``` switch from one state to
the other.

If no task has been started, ```start``` will also set the first not
started task in the list as the active task. The active task is displayed in bright green. 

If however the user is not working, it will be purple.

### Next

When the user finishes a task, ```akiv next``` moves to the next
task. Already done tasks are displayed in regular green.









