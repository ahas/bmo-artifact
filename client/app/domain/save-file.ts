// A file the service serves for saving is taken as its link would be, so the browser saves it under the name
// the service gives it.
export function saveFile(address: string): void {
  const link = document.createElement("a");

  link.href = address;
  link.download = "";
  link.click();
}
