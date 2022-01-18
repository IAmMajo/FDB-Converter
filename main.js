import "beercss";
import "./style.css";

let isFdbToSqlite = true;
let module = null;
let objectUrl = null;

const ftsChip = document.getElementById("fts-chip");
const stfChip = document.getElementById("stf-chip");
const fileElement = document.getElementById("file");
const error = document.getElementById("error");
const errorFileType = document.getElementById("error-file-type");
const errorClose = document.getElementById("error-close");
const errorFileTypeClose = document.getElementById("error-file-type-close");
const fileIcon = document.getElementById("file-icon");
const loader = document.getElementById("loader");
const dropArea = document.getElementById("drop-area");

ftsChip.addEventListener("click", () => {
  ftsChip.classList.remove("border");
  stfChip.classList.add("border");
  document
    .querySelectorAll(".fts")
    .forEach((element) => element.classList.remove("invisible"));
  document
    .querySelectorAll(".stf")
    .forEach((element) => element.classList.add("invisible"));
  fileElement.setAttribute("accept", ".fdb");
  isFdbToSqlite = true;
});

stfChip.addEventListener("click", () => {
  stfChip.classList.remove("border");
  ftsChip.classList.add("border");
  document
    .querySelectorAll(".stf")
    .forEach((element) => element.classList.remove("invisible"));
  document
    .querySelectorAll(".fts")
    .forEach((element) => element.classList.add("invisible"));
  fileElement.setAttribute("accept", ".sqlite");
  isFdbToSqlite = false;
});

dropArea.addEventListener("dragover", () =>
  dropArea.classList.remove("border")
);

dropArea.addEventListener("dragleave", () => dropArea.classList.add("border"));

errorClose.addEventListener("click", () => error.classList.remove("active"));

errorFileTypeClose.addEventListener("click", () =>
  errorFileType.classList.remove("active")
);

fileElement.addEventListener("change", async () => {
  if (objectUrl) {
    window.URL.revokeObjectURL(objectUrl);
  }
  const file = fileElement.files[0];
  fileElement.value = null;
  error.classList.remove("active");
  errorFileType.classList.remove("active");
  let inputFileExtension;
  let outputFileExtension;
  if (isFdbToSqlite) {
    inputFileExtension = "fdb";
    outputFileExtension = "sqlite";
  } else {
    inputFileExtension = "sqlite";
    outputFileExtension = "fdb";
  }
  if (!file.name.endsWith("." + inputFileExtension)) {
    errorFileType.classList.add("active");
    dropArea.classList.add("border");
    return;
  }
  fileIcon.classList.add("invisible");
  loader.classList.remove("invisible");
  fileElement.disabled = true;
  if (!module) {
    module = await Module();
  }
  const input = new Uint8Array(await file.arrayBuffer());
  module.FS.writeFile("input." + inputFileExtension, input);
  let output;
  try {
    module.ccall(inputFileExtension + "_to_" + outputFileExtension, null);
  } catch (e) {
    error.classList.add("active");
    return;
  } finally {
    module.FS.unlink("input." + inputFileExtension);
    fileIcon.classList.remove("invisible");
    loader.classList.add("invisible");
    fileElement.disabled = false;
    dropArea.classList.add("border");
  }
  output = module.FS.readFile("output." + outputFileExtension);
  module.FS.unlink("output." + outputFileExtension);
  objectUrl = window.URL.createObjectURL(
    new Blob([output], { type: "application/octet-stream" })
  );
  const a = document.createElement("a");
  a.href = objectUrl;
  a.download =
    file.name.split(".").slice(0, -1).join(".") + "." + outputFileExtension;
  document.body.appendChild(a);
  a.click();
  a.remove();
});
