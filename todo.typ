#show raw.where(block: false): it => box(move(dy:2pt, box(fill: luma(200), inset: 2pt, radius: 2pt, it)))

#let todos = it => {
  // Check for checkbox syntax: [ ]
  if repr(it.body.func()) == "sequence" and it.body.children.len() >= 3 and it.body.children.at(0) == [\[] and it.body.children.at(2) == [\]] {
    set list(marker: [])
    // Initialize item content variable
    let checkbox = []
    // Need to skip first 4 characters ([,x,],⎵)

    let text_content = []
    if it.body.children.len() >= 4 {
      text_content = it.body.children.slice(4).fold([], (acc, item) => {acc + item})
    }
    
    if it.body.children.at(1) == [ ] {
      // if not checked, add prefix for empty checkbox
      checkbox = emoji.square.white
    } else if it.body.children.at(1) == [x] {
      // If checked, add prefix for done checkbox
      checkbox = emoji.ballot
    } else if it.body.children.at(1) == [/] {
      // If checked, add prefix for done checkbox
      checkbox = emoji.square.blue
    } else if it.body.children.at(1) == [-] {
      // If checked, add prefix for done checkbox and strike content
      checkbox = emoji.ballot
      text_content = strike(text_content)
    }
    
    [#list.item(checkbox + h(0.5em) + text_content) <todo>]
  } else { // If checkbox syntax [ ] is not present, just return the original
    it
  }
}

#show list.item: todos
#set raw(lang: "Rust")

= Graphema:
- [ ] Implement traits:
  - [ ] `TryFrom<Graph>`
- [ ] Move to separate repo
- [ ] Move tests to separate file

= Spoor:
- [ ] Implement `ShortestPath` for A\*
  - [ ] Add `orientation_bias` option
  - [ ] Allow changing of next neighbour heuristic
- [ ] Implement `ShortestTree` for A\*
- [ ] Implement `ShortestTree` for Physarum
- Testing
= Lokigo:
