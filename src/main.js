import "@fontsource/roboto/latin-400.css";
import "@fontsource/roboto/latin-500.css";
import "./theme/theme.css";
import "@fontsource/material-icons";
import "@fontsource/material-icons-outlined";
import "@material/web/ripple/ripple";
import "@material/web/navigationbar/navigation-bar";
import "@material/web/navigationtab/navigation-tab";
import "@loadingio/css-spinner/entries/ring/index.css";
import "@material/web/button/text-button";
import "./style.css";
import { ActionController } from "@material/web/controller/action-controller";

document.querySelectorAll("md-navigation-tab").forEach((tab) => {
  const tabStyle = document.createElement("style");
  tabStyle.textContent =
    "@media (min-width: 600px) and (orientation: portrait), " +
    "(min-width: 916px) { span.md3-navigation-tab__label-text { height: 32px " +
    "} }";
  tab.shadowRoot.append(tabStyle);
});

const dropArea = document.querySelector("label");
const ripple = document.querySelector("md-ripple");
const controller = new ActionController(ripple);
dropArea.addEventListener("pointerenter", (event) => ripple.beginHover(event));
dropArea.addEventListener("pointerleave", (event) => {
  ripple.endHover();
  controller.pointerLeave(event);
});
dropArea.addEventListener("pointerdown", controller.pointerDown);
dropArea.addEventListener("pointerup", controller.pointerUp);
dropArea.addEventListener("click", controller.click);
dropArea.addEventListener("pointercancel", controller.pointerCancel);
dropArea.addEventListener("contextmenu", controller.contextMenu);
dropArea.addEventListener("dragover", () =>
  dropArea.classList.add("surface-variant", "on-surface-variant-text")
);
dropArea.addEventListener("dragleave", () =>
  dropArea.classList.remove("surface-variant", "on-surface-variant-text")
);

let objectUrl = null;
let module = null;
const fileElement = document.querySelector("input");
const fileIcon = document.querySelector(".file-icon");
fileElement.addEventListener("change", async () => {
  if (objectUrl) {
    window.URL.revokeObjectURL(objectUrl);
  }
  dropArea.classList.remove("surface-variant", "on-surface-variant-text");
  closeError();
  const file = fileElement.files[0];
  fileElement.value = null;
  const inputExtension = fileElement.dataset.inputExtension;
  const outputExtension = fileElement.dataset.outputExtension;
  if (!file.name.endsWith("." + inputExtension)) {
    showError("The file does not have the correct file type.");
    return;
  }
  fileIcon.style.display = "none";
  const loader = document.createElement("div");
  loader.classList.add("lds-ring");
  const emptyDiv = document.createElement("div");
  loader.appendChild(emptyDiv);
  loader.appendChild(emptyDiv.cloneNode());
  loader.appendChild(emptyDiv.cloneNode());
  loader.appendChild(emptyDiv.cloneNode());
  fileIcon.insertAdjacentElement("afterend", loader);
  fileElement.disabled = true;
  if (!module) {
    const script = document.createElement("script");
    script.src = "/fdb-converter.js";
    const scriptLoad = new Promise((resolve) =>
      script.addEventListener("load", () => resolve())
    );
    document.head.append(script);
    await scriptLoad;
    module = await Module();
  }
  const input = new Uint8Array(await file.arrayBuffer());
  module.FS.writeFile("input", input);
  try {
    module.ccall(inputExtension + "_to_" + outputExtension, null);
  } catch (e) {
    showError("An error occured while converting.");
    return;
  } finally {
    module.FS.unlink("input");
    fileIcon.style.display = "";
    loader.remove();
    fileElement.disabled = false;
  }
  const output = module.FS.readFile("output");
  module.FS.unlink("output");
  objectUrl = window.URL.createObjectURL(
    new Blob([output], { type: "application/octet-stream" })
  );
  const a = document.createElement("a");
  a.href = objectUrl;
  a.download =
    file.name.split(".").slice(0, -1).join(".") + "." + outputExtension;
  document.body.appendChild(a);
  a.click();
  a.remove();
});

function showError(message) {
  const wrapper = document.createElement("div");
  wrapper.classList.add("error-wrapper");
  const error = document.createElement("div");
  error.classList.add("error-container", "on-error-container-text");
  const p = document.createElement("p");
  p.textContent = message;
  error.appendChild(p);
  const button = document.createElement("md-text-button");
  button.label = "Close";
  error.appendChild(button);
  wrapper.appendChild(error);
  document.body.appendChild(wrapper);
  button.addEventListener("click", closeError);
}

function closeError() {
  const wrapper = document.querySelector(".error-wrapper");
  if (wrapper) {
    wrapper.remove();
  }
}
