mergeInto(LibraryManager.library, {
  BattlementDittoWebInstall: function (ownerPointer) {
    var owner = UTF8ToString(ownerPointer);
    if (globalThis.battlementDittoBrowserBridge) {
      return;
    }
    var sending = false;
    var report = function (kind, value) {
      if (sending) return;
      sending = true;
      try {
        var message = String(value == null ? "unknown browser failure" : value);
        SendMessage(owner, "ReportBrowserFailure", JSON.stringify({
          kind: kind,
          message: message.slice(0, 4096),
        }));
      } finally {
        sending = false;
      }
    };
    var originalError = console.error.bind(console);
    console.error = function () {
      originalError.apply(console, arguments);
      report("console", Array.prototype.join.call(arguments, " "));
    };
    addEventListener("error", function (event) {
      report("exception", event.error && event.error.stack || event.message);
    });
    addEventListener("unhandledrejection", function (event) {
      report("promise", event.reason && event.reason.stack || event.reason);
    });
    globalThis.battlementDittoBrowserBridge = { owner: owner, report: report };
  },

  BattlementDittoWebProbe: function (ownerPointer, width, height) {
    var owner = UTF8ToString(ownerPointer);
    var canvas = Module.canvas;
    var fail = function (reason) {
      SendMessage(owner, "CompleteWebProbe", JSON.stringify({ ok: false, reason: reason }));
    };
    try {
      if (!canvas || canvas !== document.getElementById("unity-canvas")) {
        fail("The Unity render canvas identity is invalid.");
        return;
      }
      if (canvas.width !== width || canvas.height !== height) {
        fail("The Unity canvas dimensions do not match the configured display.");
        return;
      }
      var gl = Module.ctx || GL.currentContext && GL.currentContext.GLctx;
      if (!gl || !gl.getContextAttributes().alpha) {
        fail("The Unity WebGL context does not preserve alpha.");
        return;
      }
      var scissor = gl.isEnabled(gl.SCISSOR_TEST);
      var clear = gl.getParameter(gl.COLOR_CLEAR_VALUE);
      var box = gl.getParameter(gl.SCISSOR_BOX);
      gl.enable(gl.SCISSOR_TEST);
      gl.scissor(0, 0, width, Math.max(1, Math.floor(height / 2)));
      gl.clearColor(11 / 255, 23 / 255, 47 / 255, 61 / 255);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.scissor(0, Math.floor(height / 2), width, Math.ceil(height / 2));
      gl.clearColor(131 / 255, 149 / 255, 167 / 255, 181 / 255);
      gl.clear(gl.COLOR_BUFFER_BIT);
      var bottom = new Uint8Array(4);
      var top = new Uint8Array(4);
      gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, bottom);
      gl.readPixels(width - 1, height - 1, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, top);
      gl.scissor(box[0], box[1], box[2], box[3]);
      gl.clearColor(clear[0], clear[1], clear[2], clear[3]);
      if (!scissor) gl.disable(gl.SCISSOR_TEST);
      var expectedBottom = [11, 23, 47, 61];
      var expectedTop = [131, 149, 167, 181];
      if (!expectedBottom.every(function (value, index) { return bottom[index] === value; }) ||
          !expectedTop.every(function (value, index) { return top[index] === value; })) {
        fail("The canvas probe did not preserve orientation, colors, and alpha.");
        return;
      }
      SendMessage(owner, "CompleteWebProbe", JSON.stringify({
        ok: true,
        width: width,
        height: height,
        reason: "",
      }));
    } catch (error) {
      fail(error && error.message || error);
    }
  },

  BattlementDittoWebCapture: function (
    ownerPointer,
    urlPointer,
    artifactPointer,
    width,
    height,
    frameLow,
    frameHigh
  ) {
    var owner = UTF8ToString(ownerPointer);
    var url = UTF8ToString(urlPointer);
    var artifactId = UTF8ToString(artifactPointer);
    var frame = typeof frameLow === "bigint"
      ? Number(frameLow)
      : (frameLow >>> 0) + (frameHigh >>> 0) * 4294967296;
    var fail = function (reason) {
      SendMessage(owner, "CompleteWebCapture", JSON.stringify({
        ok: false,
        artifactId: artifactId,
        sha256: "",
        width: width,
        height: height,
        frame: frame,
        reason: String(reason).slice(0, 4096),
      }));
    };
    requestAnimationFrame(function () {
      Module.canvas.toBlob(async function (blob) {
        if (!blob) {
          fail("The browser returned a null PNG blob.");
          return;
        }
        try {
          var bytes = await blob.arrayBuffer();
          var digest = await crypto.subtle.digest("SHA-256", bytes);
          var sha256 = Array.from(new Uint8Array(digest))
            .map(function (value) { return value.toString(16).padStart(2, "0"); })
            .join("");
          var response = await fetch(url, {
            method: "PUT",
            headers: {
              "Content-Type": "image/png",
              "X-Ditto-SHA256": sha256,
              "X-Ditto-Width": String(width),
              "X-Ditto-Height": String(height),
            },
            body: blob,
          });
          if (!response.ok) {
            fail("PNG upload returned HTTP " + response.status + ": " + await response.text());
            return;
          }
          var acknowledgement = await response.json();
          if (acknowledgement.artifact_id !== artifactId || acknowledgement.sha256 !== sha256) {
            fail("PNG upload returned an invalid artifact acknowledgement.");
            return;
          }
          SendMessage(owner, "CompleteWebCapture", JSON.stringify({
            ok: true,
            artifactId: artifactId,
            sha256: sha256,
            width: width,
            height: height,
            frame: frame,
            reason: "",
          }));
        } catch (error) {
          fail(error && error.message || error);
        }
      }, "image/png");
    });
  },
});
