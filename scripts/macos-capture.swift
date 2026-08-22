import AVFoundation
import AppKit
import ApplicationServices
import CoreGraphics
import CoreMedia
import CoreVideo
import Foundation
import ScreenCaptureKit

private final class WindowRecorder: NSObject, SCStreamOutput {
    let adaptor: AVAssetWriterInputPixelBufferAdaptor
    let input: AVAssetWriterInput
    let writer: AVAssetWriter
    private let lock = NSLock()
    private var latestBuffer: CVPixelBuffer?

    init(path: String, width: Int, height: Int) throws {
        writer = try AVAssetWriter(outputURL: URL(fileURLWithPath: path), fileType: .mp4)
        input = AVAssetWriterInput(mediaType: .video, outputSettings: [
            AVVideoCodecKey: AVVideoCodecType.h264,
            AVVideoWidthKey: width,
            AVVideoHeightKey: height,
        ])
        input.expectsMediaDataInRealTime = true
        guard writer.canAdd(input) else {
            throw NSError(domain: "BattlementCapture", code: 1,
                userInfo: [NSLocalizedDescriptionKey: "Cannot configure H.264 writer."])
        }
        writer.add(input)
        adaptor = AVAssetWriterInputPixelBufferAdaptor(
            assetWriterInput: input,
            sourcePixelBufferAttributes: [
                kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
                kCVPixelBufferWidthKey as String: width,
                kCVPixelBufferHeightKey as String: height,
            ])
    }

    func stream(_ stream: SCStream, didOutputSampleBuffer sampleBuffer: CMSampleBuffer,
                of type: SCStreamOutputType) {
        guard type == .screen, sampleBuffer.isValid else { return }
        guard let attachments = CMSampleBufferGetSampleAttachmentsArray(
            sampleBuffer, createIfNecessary: false) as? [[SCStreamFrameInfo: Any]],
            let statusValue = attachments.first?[.status] as? Int,
            statusValue == SCFrameStatus.complete.rawValue,
            let buffer = sampleBuffer.imageBuffer else {
            return
        }
        lock.lock()
        latestBuffer = buffer
        lock.unlock()
    }

    func buffer() -> CVPixelBuffer? {
        lock.lock()
        defer { lock.unlock() }
        return latestBuffer
    }
}

private func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(1)
}

private func argument(_ index: Int) -> String {
    guard CommandLine.arguments.indices.contains(index) else {
        fail("Missing argument for \(CommandLine.arguments.first ?? "capture helper").")
    }
    return CommandLine.arguments[index]
}

private func window(for processIdentifier: pid_t) -> [String: Any]? {
    let options: CGWindowListOption = [.optionOnScreenOnly, .excludeDesktopElements]
    guard let windows = CGWindowListCopyWindowInfo(options, kCGNullWindowID)
        as? [[String: Any]] else {
        return nil
    }
    return windows.filter { item in
        guard let owner = item[kCGWindowOwnerPID as String] as? NSNumber,
              let layer = item[kCGWindowLayer as String] as? NSNumber,
              let bounds = item[kCGWindowBounds as String] as? [String: Any],
              let width = bounds["Width"] as? NSNumber,
              let height = bounds["Height"] as? NSNumber else {
            return false
        }
        return owner.int32Value == processIdentifier && layer.intValue == 0
            && width.intValue > 100 && height.intValue > 100
    }.max { left, right in
        bounds(from: left).width * bounds(from: left).height
            < bounds(from: right).width * bounds(from: right).height
    }
}

private func bounds(from item: [String: Any]) -> CGRect {
    guard let dictionary = item[kCGWindowBounds as String] as? NSDictionary,
          let bounds = CGRect(dictionaryRepresentation: dictionary) else {
        fail("The player window did not provide valid bounds.")
    }
    return bounds
}

private func inspectVideo(at path: String) {
    let asset = AVURLAsset(url: URL(fileURLWithPath: path))
    let semaphore = DispatchSemaphore(value: 0)
    var result: String?
    var failure: Error?
    Task {
        do {
            guard let track = try await asset.loadTracks(withMediaType: .video).first else {
                fail("Captured video has no video track.")
            }
            let size = try await track.load(.naturalSize)
            let frameRate = try await track.load(.nominalFrameRate)
            let descriptions = try await track.load(.formatDescriptions)
            guard let description = descriptions.first else {
                fail("Captured video has no format description.")
            }
            let codec = CMFormatDescriptionGetMediaSubType(description)
            let codecText = String(format: "%c%c%c%c",
                (codec >> 24) & 0xff, (codec >> 16) & 0xff,
                (codec >> 8) & 0xff, codec & 0xff)
            result = "\(Int(size.width)) \(Int(size.height)) \(frameRate) \(codecText)"
        } catch {
            failure = error
        }
        semaphore.signal()
    }
    semaphore.wait()
    if let failure {
        fail("Could not inspect captured video: \(failure)")
    }
    print(result ?? "")
}

private func recordWindow(identifier: CGWindowID, path: String, seconds: Double,
                          width: Int, height: Int, readyPath: String) async throws {
    let content = try await SCShareableContent.excludingDesktopWindows(
        true, onScreenWindowsOnly: true)
    guard let window = content.windows.first(where: { $0.windowID == identifier }) else {
        fail("The selected player window is unavailable to ScreenCaptureKit.")
    }
    let filter = SCContentFilter(desktopIndependentWindow: window)
    let configuration = SCStreamConfiguration()
    configuration.width = width
    configuration.height = height
    configuration.minimumFrameInterval = CMTime(value: 1, timescale: 30)
    configuration.queueDepth = 6
    configuration.capturesAudio = false
    configuration.showsCursor = true

    let recorder = try WindowRecorder(path: path, width: width, height: height)
    let stream = SCStream(filter: filter, configuration: configuration, delegate: nil)
    let queue = DispatchQueue(label: "com.battlement.capture.video")
    try stream.addStreamOutput(recorder, type: .screen, sampleHandlerQueue: queue)
    guard recorder.writer.startWriting() else {
        throw recorder.writer.error ?? NSError(domain: "BattlementCapture", code: 2,
            userInfo: [NSLocalizedDescriptionKey: "H.264 writer could not start."])
    }
    recorder.writer.startSession(atSourceTime: .zero)
    try await stream.startCapture()
    for _ in 0..<2_500 {
        if recorder.buffer() != nil { break }
        try await Task.sleep(for: .milliseconds(2))
    }
    guard recorder.buffer() != nil else {
        throw NSError(domain: "BattlementCapture", code: 4,
            userInfo: [NSLocalizedDescriptionKey: "Capture stream produced no initial frame."])
    }
    try Data().write(to: URL(fileURLWithPath: readyPath), options: .atomic)
    let frameCount = Int(seconds * 30)
    for frame in 0..<frameCount {
        try await Task.sleep(for: .seconds(1.0 / 30.0))
        guard let buffer = recorder.buffer() else { continue }
        while !recorder.input.isReadyForMoreMediaData {
            try await Task.sleep(for: .milliseconds(2))
        }
        if !recorder.adaptor.append(buffer,
            withPresentationTime: CMTime(value: Int64(frame), timescale: 30)) {
            throw recorder.writer.error ?? NSError(domain: "BattlementCapture", code: 3,
                userInfo: [NSLocalizedDescriptionKey: "Could not append a captured frame."])
        }
    }
    try await stream.stopCapture()
    recorder.input.markAsFinished()
    await recorder.writer.finishWriting()
    if recorder.writer.status != .completed {
        throw recorder.writer.error ?? NSError(domain: "BattlementCapture", code: 2,
            userInfo: [NSLocalizedDescriptionKey: "H.264 writer did not complete."])
    }
}

switch argument(1) {
case "launch-background":
    let configuration = NSWorkspace.OpenConfiguration()
    configuration.activates = false
    configuration.createsNewApplicationInstance = true
    configuration.arguments = Array(CommandLine.arguments.dropFirst(3))
    configuration.environment = ProcessInfo.processInfo.environment.filter {
        $0.key != "DYLD_LIBRARY_PATH" && $0.key != "DYLD_FRAMEWORK_PATH"
    }
    let semaphore = DispatchSemaphore(value: 0)
    var launchedProcess: pid_t?
    var launchFailure: Error?
    NSWorkspace.shared.openApplication(
        at: URL(fileURLWithPath: argument(2)),
        configuration: configuration
    ) { application, error in
        launchedProcess = application?.processIdentifier
        launchFailure = error
        semaphore.signal()
    }
    semaphore.wait()
    if let launchFailure {
        fail("Could not launch the capture player: \(launchFailure)")
    }
    guard let launchedProcess else {
        fail("The capture player launch returned no process.")
    }
    print(launchedProcess)
case "preflight":
    guard CGPreflightScreenCaptureAccess() else {
        fail("Screen Recording permission is required for the capture command.")
    }
    guard AXIsProcessTrusted() else {
        fail("Accessibility permission is required to drive player pointer input.")
    }
case "preflight-input":
    guard AXIsProcessTrusted() else {
        fail("Accessibility permission is required to drive player pointer input.")
    }
case "window":
    guard let processIdentifier = pid_t(argument(2)),
          let item = window(for: processIdentifier),
          let number = item[kCGWindowNumber as String] as? NSNumber else {
        fail("No visible player content window was found.")
    }
    let frame = bounds(from: item)
    print("\(number.uint32Value) \(Int(frame.minX)) \(Int(frame.minY)) "
        + "\(Int(frame.width)) \(Int(frame.height))")
case "focus":
    guard let processIdentifier = pid_t(argument(2)),
          let application = NSRunningApplication(processIdentifier: processIdentifier) else {
        fail("Could not focus the capture player.")
    }
    application.activate(options: [.activateAllWindows])
case "pointer-move", "pointer-left-drag", "pointer-left-button-down",
     "pointer-left-button-up":
    guard pid_t(argument(2)) != nil,
          let x = Double(argument(3)), let y = Double(argument(4)) else {
        fail("Pointer coordinates must be numeric.")
    }
    let point = CGPoint(x: x, y: y)
    let eventType: CGEventType
    switch argument(1) {
    case "pointer-move": eventType = .mouseMoved
    case "pointer-left-drag": eventType = .leftMouseDragged
    case "pointer-left-button-down": eventType = .leftMouseDown
    default: eventType = .leftMouseUp
    }
    guard let event = CGEvent(mouseEventSource: nil, mouseType: eventType,
                              mouseCursorPosition: point, mouseButton: .left) else {
        fail("Could not create the requested pointer event.")
    }
    event.post(tap: .cghidEventTap)
case "record-window":
    guard let identifier = CGWindowID(argument(2)),
          let seconds = Double(argument(4)),
          let width = Int(argument(5)),
          let height = Int(argument(6)) else {
        fail("Window recording arguments are invalid.")
    }
    _ = NSApplication.shared
    let semaphore = DispatchSemaphore(value: 0)
    var recordingFailure: Error?
    Task {
        do {
            try await recordWindow(identifier: identifier, path: argument(3),
                seconds: seconds, width: width, height: height,
                readyPath: argument(7))
        } catch {
            recordingFailure = error
        }
        semaphore.signal()
    }
    semaphore.wait()
    if let recordingFailure {
        fail("Window recording failed: \(recordingFailure)")
    }
case "inspect-video":
    inspectVideo(at: argument(2))
default:
    fail("Unknown capture helper command: \(argument(1))")
}
