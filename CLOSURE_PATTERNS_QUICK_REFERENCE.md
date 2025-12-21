# Closure Capture Patterns - Quick Reference

## Problem Patterns & Solutions

### Pattern 1: Multiple Closures Capturing Same Value

**Problem:**
```rust
let data = vec![...];
let memo1 = Memo::new(move |_| {
    let sorted = data.clone();  // moved
});
let memo2 = Memo::new(move |_| {
    (data.len() - 1)  // ERROR: moved value
});
```

**Solution - Clone Before Each Capture:**
```rust
let data_copy1 = data.clone();
let data_copy2 = data.clone();
let memo1 = Memo::new(move |_| {
    let sorted = data_copy1.clone();
});
let memo2 = Memo::new(move |_| {
    (data_copy2.len() - 1)
});
```

---

### Pattern 2: Non-Copy Callback in Multiple Event Handlers

**Problem:**
```rust
// impl Fn doesn't implement Copy
fn MyComponent(on_submit: impl Fn(String) + 'static) {
    let keydown = move |ev| {
        on_submit(text);  // moved
    };
    
    // Button click
    on_click=move |_| {
        on_submit(text);  // ERROR: moved value
    }
}
```

**Solution - Wrap with Rc & Clone:**
```rust
use std::rc::Rc;

fn MyComponent(on_submit: impl Fn(String) + 'static) {
    let on_submit = Rc::new(on_submit);
    
    let on_submit_keydown = on_submit.clone();
    let keydown = move |ev| {
        on_submit_keydown(text);
    };
    
    let on_click_submit = on_submit.clone();
    on_click=Box::new(move |_| {
        on_click_submit(text);
    })
}
```

---

### Pattern 3: Value Moved by Iterator, Needed Later

**Problem:**
```rust
let items = vec![...];
{items.into_iter().enumerate().map(|...| ...)}  // moved
{items.is_empty().then(||...)}  // ERROR: moved
```

**Solution - Check/Use Before Moving:**
```rust
let items_empty = items.is_empty();
{items.into_iter().enumerate().map(|...| ...)}
{items_empty.then(||...)}
```

OR **Use Borrowed Iterator:**
```rust
{items.iter().enumerate().map(|(idx, item)| {
    let item = item.clone();  // now can use item
})}
{items.is_empty().then(||...)}  // items not moved!
```

---

### Pattern 4: Dyn Fn Callback in Multiple Closures

**Problem:**
```rust
// Option<Box<dyn Fn>> doesn't implement Copy
fn Component(on_action: Option<Box<dyn Fn(Action)>>) {
    // click1
    on:click=move |_| {
        if let Some(cb) = on_action {  // moved
            cb(Action::A);
        }
    }
    
    // click2
    on:click=move |_| {
        if let Some(cb) = on_action {  // ERROR: moved
            cb(Action::B);
        }
    }
}
```

**Solution - Wrap with Rc:**
```rust
use std::rc::Rc;

fn Component(on_action: Option<Box<dyn Fn(Action)>>) {
    let on_action = Rc::new(on_action);
    
    // click1 - uses on_action via Rc
    on:click=move |_| {
        if let Some(ref cb) = *on_action {
            cb(Action::A);
        }
    }
    
    // click2 - uses on_action via Rc (cloned for ownership)
    let on_action_2 = on_action.clone();
    on:click=move |_| {
        if let Some(ref cb) = *on_action_2 {
            cb(Action::B);
        }
    }
}
```

---

### Pattern 5: Different View Types in Branches

**Problem:**
```rust
if condition {
    view! { <RenderText /> }.into_view()
} else {
    view! { <div>...</div> }.into_view()
}  // ERROR: different view types!
```

**Solution - Wrap with Fragment:**
```rust
if condition {
    view! { <> <RenderText /> </> }.into_view()
} else {
    view! { <> <div>...</div> </> }.into_view()
}
```

---

### Pattern 6: Closures Not Implementing Copy Trait

**Problem:**
```rust
let callback = |text| on_submit(text);  // closure
let handler1 = callback;  // move
let handler2 = callback;  // ERROR: moved
```

**Solution:**
- **Avoid naming closures** - pass inline
- **Use Rc if must reuse**: `Rc::new(move |...| ...)`
- **Inline logic** - duplicate if only 2-3 uses

---

## Leptos-Specific Patterns

### Event Handler Closures

**Correct signature:**
```rust
on:click=move |ev: web_sys::MouseEvent| { ... }  // takes event
on:change=move |ev: web_sys::Event| { ... }       // takes event
on:keydown=move |ev: web_sys::KeyboardEvent| { ... }
```

**Wrong signature:**
```rust
on:click=move || { ... }  // ERROR: expects 0 args, takes 1
```

### Optional Event Handlers

Must be `Box<dyn Fn>`:
```rust
on_click: Option<Box<dyn Fn(web_sys::MouseEvent)>>

// Usage:
on:click=move |ev| {
    if let Some(ref handler) = on_click {
        handler(ev);
    }
}
```

---

## Summary: When to Use What

| Situation | Solution | Example |
|-----------|----------|---------|
| Multiple closures need same value | Clone before each | `let v1 = v.clone(); let v2 = v.clone();` |
| Non-Copy callback in 2+ places | Rc + clone | `let cb = Rc::new(cb);` |
| Value moved by iterator, used later | Check before moving | `let is_empty = v.is_empty(); v.into_iter()...` |
| Different view types in if/else | Fragment wrapper | `view! { <> ... </> }` |
| Closure needs reuse | Inline or Rc | Avoid storing closures |
| Signature mismatch | Check event param | `move \|ev\|` not `move \|\|` |

