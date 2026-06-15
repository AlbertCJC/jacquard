// jacquard_runtime.h — Cooperative task runtime for Jacquard-compiled code.
//
// Generated C++ code links against this header. It provides:
//   - Poll<T>       — result-or-pending wrapper for async operations
//   - Task          — abstract base for state-machine tasks
//   - Runtime       — cooperative scheduler: spawn, tick, panic hook
//
// Target: C++11 or later. No platform-specific assumptions.

#ifndef JACQUARD_RUNTIME_H
#define JACQUARD_RUNTIME_H

#include <cstdint>
#include <functional>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

namespace jq {

// ---------------------------------------------------------------------------
// Poll — wraps an async result that may still be pending
// ---------------------------------------------------------------------------

template <typename T>
struct Poll {
    bool ready;
    T value;

    Poll() : ready(false), value{} {}
    explicit Poll(T v) : ready(true), value(std::move(v)) {}

    /// Returns true if the operation has completed.
    bool is_ready() const { return ready; }

    /// Returns true if the operation is still in progress.
    bool is_pending() const { return !ready; }
};

// Specialization for void-returning operations.
template <>
struct Poll<void> {
    bool ready;

    Poll() : ready(false) {}
    explicit Poll(bool r) : ready(r) {}

    bool is_ready() const { return ready; }
    bool is_pending() const { return !ready; }
};

// ---------------------------------------------------------------------------
// Task — abstract base for compiled task state machines
// ---------------------------------------------------------------------------

struct Task {
    /// Advance the task's state machine by one step.
    ///
    /// `dt` is the delta time in seconds since the last tick.
    /// Returns `true` when the task has completed (success or error).
    /// Returns `false` while the task still has work to do.
    virtual bool tick(float dt) = 0;

    virtual ~Task() = default;
};

// ---------------------------------------------------------------------------
// Runtime — cooperative task scheduler
// ---------------------------------------------------------------------------

class Runtime {
public:
    using PanicHook = std::function<void(const char* file, int line, const char* message)>;

    Runtime() : panic_hook_(default_panic_hook) {}

    // -- Task management -----------------------------------------------------

    /// Spawn a task by type, forwarding constructor arguments.
    ///
    /// Returns a non-owning pointer. The runtime owns the task and will
    /// delete it after completion.
    template <typename T, typename... Args>
    T* spawn(Args&&... args) {
        auto task = std::make_unique<T>(std::forward<Args>(args)...);
        T* ptr = task.get();
        tasks_.push_back(std::move(task));
        return ptr;
    }

    /// Number of currently active (incomplete) tasks.
    size_t active_count() const {
        size_t count = 0;
        for (auto& t : tasks_) {
            if (t) ++count;
        }
        return count;
    }

    /// Total number of tasks spawned (including completed ones).
    size_t total_spawned() const { return total_spawned_; }

    // -- Main loop -----------------------------------------------------------

    /// Tick all active tasks once.
    ///
    /// Completed tasks are removed. `delta_time` is in seconds.
    /// Returns the number of tasks that completed during this tick.
    size_t tick(float delta_time) {
        size_t completed = 0;
        for (auto& task : tasks_) {
            if (!task) continue;
            if (task->tick(delta_time)) {
                task.reset(); // completed — free it
                ++completed;
            }
        }
        // Compact the task list periodically.
        if (completed > 0) {
            compact();
        }
        return completed;
    }

    /// Tick until all active tasks complete, or `timeout_seconds` elapses.
    ///
    /// `delta_time` is the fixed step size for each tick.
    /// Returns the number of remaining active tasks (0 = all done).
    size_t run(float delta_time, float timeout_seconds = 60.0f) {
        float elapsed = 0.0f;
        while (active_count() > 0 && elapsed < timeout_seconds) {
            tick(delta_time);
            elapsed += delta_time;
        }
        return active_count();
    }

    // -- Panic hook ----------------------------------------------------------

    /// Set a custom panic handler. Called when a compiled task or runtime
    /// encounters an unrecoverable error.
    void set_panic_hook(PanicHook hook) {
        panic_hook_ = std::move(hook);
    }

    /// Trigger the panic hook. Called by generated code.
    void panic(const char* file, int line, const char* message) {
        if (panic_hook_) {
            panic_hook_(file, line, message);
        }
    }

private:
    std::vector<std::unique_ptr<Task>> tasks_;
    size_t total_spawned_ = 0;
    PanicHook panic_hook_;

    /// Remove null entries from the task list.
    void compact() {
        tasks_.erase(
            std::remove_if(
                tasks_.begin(),
                tasks_.end(),
                [](const std::unique_ptr<Task>& t) { return !t; }
            ),
            tasks_.end()
        );
    }

    static void default_panic_hook(const char* file, int line, const char* message) {
        fprintf(stderr, "Jacquard panic at %s:%d: %s\n", file, line, message);
        std::abort();
    }
};

// ---------------------------------------------------------------------------
// Error handling helpers
// ---------------------------------------------------------------------------

/// A simple Result type used by the `?` operator lowering.
template <typename T, typename E>
struct Result {
    enum class Tag { Ok, Err };
    Tag tag;
    union {
        T ok;
        E err;
    };

    Result() : tag(Tag::Ok), ok{} {}

    static Result Ok(T value) {
        Result r;
        r.tag = Tag::Ok;
        r.ok = std::move(value);
        return r;
    }

    static Result Err(E error) {
        Result r;
        r.tag = Tag::Err;
        r.err = std::move(error);
        return r;
    }

    bool is_ok() const { return tag == Tag::Ok; }
    bool is_err() const { return tag == Tag::Err; }

    T unwrap() const {
        if (tag == Tag::Err) {
            throw std::runtime_error("Result::unwrap() called on Err");
        }
        return ok;
    }

    E unwrap_err() const {
        if (tag == Tag::Ok) {
            throw std::runtime_error("Result::unwrap_err() called on Ok");
        }
        return err;
    }

    ~Result() {
        // Manual destruction of the active union member.
        // In C++11 we rely on trivial destructors for T and E.
        // A C++17 version would use std::variant.
    }
};

} // namespace jq

#endif // JACQUARD_RUNTIME_H