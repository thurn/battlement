const source = document.querySelector("#source");

fetch("/api/gallery")
  .then((response) => {
    if (!response.ok) throw new Error(`Gallery request failed: ${response.status}`);
    return response.json();
  })
  .then(render)
  .catch((error) => {
    source.textContent = error.message;
    source.classList.add("error");
  });

function render(gallery) {
  document.title = `${gallery.suite} · Ditto Gallery`;
  document.querySelector("#suite-name").textContent = gallery.suite;
  document.querySelector("#profile-name").textContent = gallery.profile;
  document.querySelector("#filename").textContent = gallery.filename;
  document.querySelector("#checkpoint-count").textContent =
    `${gallery.checkpoints.length} canonical screenshot${gallery.checkpoints.length === 1 ? "" : "s"}`;

  const checkpoints = new Map();
  for (const checkpoint of gallery.checkpoints) {
    const entries = checkpoints.get(checkpoint.after_line) || [];
    entries.push(checkpoint);
    checkpoints.set(checkpoint.after_line, entries);
  }
  const lines = gallery.source.split("\n");
  lines.forEach((text, index) => {
    source.append(sourceLine(text, index + 1));
    for (const checkpoint of checkpoints.get(index + 1) || []) {
      source.append(checkpointCard(checkpoint, gallery.profile));
    }
  });
}

function sourceLine(text, number) {
  const row = document.createElement("div");
  row.className = "source-line";
  const gutter = document.createElement("span");
  gutter.className = "line-number";
  gutter.textContent = number;
  const code = document.createElement("code");
  code.append(...highlight(text));
  row.append(gutter, code);
  return row;
}

function highlight(text) {
  const comment = commentStart(text);
  const code = comment < 0 ? text : text.slice(0, comment);
  const nodes = tokenize(code);
  if (comment >= 0) nodes.push(token(text.slice(comment), "comment"));
  return nodes;
}

function tokenize(text) {
  if (/^\s*\[/.test(text)) return [token(text, "section")];
  const equals = unquotedIndex(text, "=");
  const nodes = [];
  let value = text;
  if (equals >= 0) {
    nodes.push(token(text.slice(0, equals), "key"), document.createTextNode("="));
    value = text.slice(equals + 1);
  }
  const pattern = /("(?:\\.|[^"\\])*"|'[^']*'|\b(?:true|false)\b|\b\d+(?:\.\d+)?\b)/g;
  let offset = 0;
  for (const match of value.matchAll(pattern)) {
    nodes.push(document.createTextNode(value.slice(offset, match.index)));
    const kind = /^['"]/.test(match[0]) ? "string" : /true|false/.test(match[0]) ? "boolean" : "number";
    nodes.push(token(match[0], kind));
    offset = match.index + match[0].length;
  }
  nodes.push(document.createTextNode(value.slice(offset)));
  return nodes;
}

function checkpointCard(checkpoint, profile) {
  const card = document.querySelector("#checkpoint-template").content.firstElementChild.cloneNode(true);
  card.querySelector(".scenario").textContent = checkpoint.scenario;
  card.querySelector(".checkpoint-name").textContent = checkpoint.checkpoint;
  card.querySelector(".dimensions").textContent = checkpoint.width
    ? `${checkpoint.width} × ${checkpoint.height} · ${profile}`
    : `${profile} · baseline missing`;
  const shell = card.querySelector(".image-shell");
  if (checkpoint.image) {
    const image = document.createElement("img");
    image.src = checkpoint.image;
    image.alt = `${checkpoint.scenario}: ${checkpoint.checkpoint}`;
    image.loading = "lazy";
    shell.append(image);
  } else {
    shell.classList.add("missing");
    shell.textContent = "No canonical screenshot is recorded for this profile.";
  }
  return card;
}

function commentStart(text) {
  return unquotedIndex(text, "#");
}

function unquotedIndex(text, target) {
  let quote = null;
  let escaped = false;
  for (let index = 0; index < text.length; index += 1) {
    const character = text[index];
    if (escaped) {
      escaped = false;
    } else if (character === "\\" && quote === '"') {
      escaped = true;
    } else if (quote && character === quote) {
      quote = null;
    } else if (!quote && (character === '"' || character === "'")) {
      quote = character;
    } else if (!quote && character === target) {
      return index;
    }
  }
  return -1;
}

function token(text, kind) {
  const span = document.createElement("span");
  span.className = `syntax-${kind}`;
  span.textContent = text;
  return span;
}
