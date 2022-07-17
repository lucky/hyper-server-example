# Hyper server with database example

This is an example [Hyper](https://hyper.rs) server that creates and uses a PostgreSQL connection. This sounds trivial, but due to borrow-checking and my own lack of experience with Rust, this took a lot of trial and error for me to figure out. Since I couldn't find a reasonably simple example elsewhere, I decided to publish this here for future reference.

**This has not been vetted by true Rust developers, so don't consider any of this to be a "best practice".** 

## Building

Build with `cargo build`.

## Running

 1. Create the table in a database
    ```sql
    create table tasks (
      id SERIAL,
      person TEXT,
      description TEXT,
      created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
      completed_at TIMESTAMP WITH TIME ZONE NULL
    );
    ```
 2. Set the `DATABASE_URL` environment variable (example: `postgres://postgres:password@localhost:7432/task_example`)
 3. Run with `cargo run`

You can submit new tasks via curl, for example:

```sh
curl -iX POST http://localhost:3030/tasks \
  -H 'Content-Type: application/json' \
  -d '{"person": "alex", "description": "walk the dog"}'
```

And then view subsequent tasks at `http://localhost:3030/tasks`:

```sh
curl http://localhost:3030/tasks
[{"id":1,"person":"alex","description":"walk the dog","created_at":"2022-07-17T22:13:14.819717Z","completed_at":null}]
```

Complete a task by submitting a `POST` request to `/tasks/<id>/complete`. For example:

```sh
curl -iX POST http://localhost:3030/tasks/1/complete
```

## Testing

There are no tests yet.
