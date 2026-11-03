#import <AVFoundation/AVFoundation.h>
#import <AppKit/AppKit.h>
#import <CoreImage/CoreImage.h>
#import <CoreVideo/CoreVideo.h>
#include <stdatomic.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>

typedef void (*CCPermissionCallback)(void *, int);
typedef void (*CCFrameCallback)(void *, const unsigned char *, size_t);
static NSArray<AVCaptureDevice *> *CCDevices(void) {
    NSMutableArray *types=[NSMutableArray arrayWithObject:AVCaptureDeviceTypeBuiltInWideAngleCamera];
    // Resolve newer device types from the installed framework, retaining compatibility
    // with older macOS SDKs and USB drivers that use ExternalUnknown.
    NSString * const *external= (NSString * const __unsafe_unretained *)dlsym(RTLD_DEFAULT,"AVCaptureDeviceTypeExternal");
    NSString * const *continuity= (NSString * const __unsafe_unretained *)dlsym(RTLD_DEFAULT,"AVCaptureDeviceTypeContinuityCamera");
    NSString * const *desk= (NSString * const __unsafe_unretained *)dlsym(RTLD_DEFAULT,"AVCaptureDeviceTypeDeskViewCamera");
    if (external && *external) [types addObject:*external];
    else {
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
        [types addObject:AVCaptureDeviceTypeExternalUnknown];
#pragma clang diagnostic pop
    }
    if (continuity && *continuity) [types addObject:*continuity];
    if (desk && *desk) [types addObject:*desk];
    NSMutableArray *found=[NSMutableArray array];
    NSMutableSet *seen=[NSMutableSet set];
    AVCaptureDeviceDiscoverySession *discovery=[AVCaptureDeviceDiscoverySession discoverySessionWithDeviceTypes:types mediaType:AVMediaTypeVideo position:AVCaptureDevicePositionUnspecified];
    for (AVCaptureDevice *device in discovery.devices) if (device.connected && ![seen containsObject:device.uniqueID]) {[found addObject:device];[seen addObject:device.uniqueID];}
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
    // Some older USB and virtual-camera drivers only appear through this fallback.
    for (AVCaptureDevice *device in [AVCaptureDevice devicesWithMediaType:AVMediaTypeVideo]) if (device.connected && ![seen containsObject:device.uniqueID]) {[found addObject:device];[seen addObject:device.uniqueID];}
#pragma clang diagnostic pop
    return found;
}
char *commons_camera_list(void) {
    @autoreleasepool {
        @try {
            NSInteger status=[AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeVideo];
            NSMutableArray *devices=[NSMutableArray array];
            for (AVCaptureDevice *device in CCDevices()) [devices addObject:@{@"id":device.uniqueID,@"name":device.localizedName,@"kind":device.deviceType}];
            NSDictionary *result=@{@"authorization":@(status),@"devices":devices,@"error":@""};
            NSData *data=[NSJSONSerialization dataWithJSONObject:result options:0 error:nil];
            return strdup([[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding].UTF8String);
        } @catch (NSException *e) {
            NSDictionary *result=@{@"authorization":@(-1),@"devices":@[],@"error":e.reason ?: @"Camera discovery failed."};
            NSData *data=[NSJSONSerialization dataWithJSONObject:result options:0 error:nil];
            return strdup([[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding].UTF8String);
        }
    }
}
void commons_camera_permission(CCPermissionCallback callback, void *context) {
    @autoreleasepool {
        AVAuthorizationStatus status=[AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeVideo];
        if (status==AVAuthorizationStatusAuthorized) {callback(context,1);return;}
        if (status==AVAuthorizationStatusDenied || status==AVAuthorizationStatusRestricted) {callback(context,0);return;}
        [AVCaptureDevice requestAccessForMediaType:AVMediaTypeVideo completionHandler:^(BOOL granted){callback(context,granted?1:0);}];
    }
}
@interface CCCamera : NSObject <AVCaptureVideoDataOutputSampleBufferDelegate> {
@public atomic_bool active;
}
@property(nonatomic,strong) AVCaptureSession *session;
@property(nonatomic,strong) AVCaptureVideoDataOutput *output;
@property(nonatomic,strong) CIContext *imageContext;
@property(nonatomic,strong) dispatch_queue_t queue;
@property(nonatomic,assign) CCFrameCallback callback;
@property(nonatomic,assign) void *context;
@property(nonatomic,assign) CFAbsoluteTime lastFrame;
@end
@implementation CCCamera
-(void)captureOutput:(AVCaptureOutput *)output didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer fromConnection:(AVCaptureConnection *)connection {
    if (!atomic_load(&active)) return;
    CFAbsoluteTime now=CFAbsoluteTimeGetCurrent();
    if (now-self.lastFrame<0.16) return;
    self.lastFrame=now;
    @autoreleasepool {
        CVImageBufferRef buffer=CMSampleBufferGetImageBuffer(sampleBuffer);
        if (!buffer) return;
        CIImage *image=[CIImage imageWithCVPixelBuffer:buffer];
        CGRect extent=image.extent;
        if (extent.size.width<1 || extent.size.height<1) return;
        CGFloat scale=MAX(320.0/extent.size.width,240.0/extent.size.height);
        image=[image imageByApplyingTransform:CGAffineTransformMakeScale(scale,scale)];
        CGRect crop=CGRectMake((image.extent.size.width-320.0)/2.0,(image.extent.size.height-240.0)/2.0,320,240);
        CGImageRef cg=[self.imageContext createCGImage:image fromRect:crop];
        if (!cg) return;
        NSBitmapImageRep *bitmap=[[NSBitmapImageRep alloc] initWithCGImage:cg];
        NSData *jpeg=[bitmap representationUsingType:NSBitmapImageFileTypeJPEG properties:@{NSImageCompressionFactor:@0.65}];
        CGImageRelease(cg);
        if (jpeg && atomic_load(&active)) self.callback(self.context,jpeg.bytes,jpeg.length);
    }
}
@end
void *commons_camera_start(const char *identifier, CCFrameCallback callback,void *context,char **error) {
    @autoreleasepool {
        @try {
            if ([AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeVideo]!=AVAuthorizationStatusAuthorized) {*error=strdup("Camera access is not authorized. Enable access in Commons or macOS Camera privacy settings.");return NULL;}
            NSString *requested=identifier ? [NSString stringWithUTF8String:identifier] : @"0";
            NSArray *devices=CCDevices(); AVCaptureDevice *selected=nil;
            if (requested.length==0 || [requested isEqualToString:@"0"] || [requested isEqualToString:@"system"]) {
                selected=[AVCaptureDevice defaultDeviceWithMediaType:AVMediaTypeVideo];
                if (!selected) selected=devices.firstObject;
            } else {
                for (AVCaptureDevice *device in devices) if ([device.uniqueID isEqualToString:requested]) {selected=device;break;}
            }
            if (!selected) {*error=strdup("The selected camera is not connected. Reconnect the USB webcam, refresh devices, and select it again.");return NULL;}
            NSError *nativeError=nil;
            AVCaptureDeviceInput *input=[AVCaptureDeviceInput deviceInputWithDevice:selected error:&nativeError];
            if (!input) {*error=strdup((nativeError.localizedDescription ?: @"Could not open this camera.").UTF8String);return NULL;}
            CCCamera *capture=[CCCamera new];
            atomic_init(&capture->active,true);
            capture.callback=callback;capture.context=context;
            capture.imageContext=[CIContext contextWithOptions:@{kCIContextUseSoftwareRenderer:@NO}];
            capture.queue=dispatch_queue_create("dev.craftanything.commons.camera",DISPATCH_QUEUE_SERIAL);
            capture.session=[AVCaptureSession new];capture.output=[AVCaptureVideoDataOutput new];
            [capture.session beginConfiguration];
            if ([capture.session canSetSessionPreset:AVCaptureSessionPreset640x480]) capture.session.sessionPreset=AVCaptureSessionPreset640x480;
            else if ([capture.session canSetSessionPreset:AVCaptureSessionPresetMedium]) capture.session.sessionPreset=AVCaptureSessionPresetMedium;
            if (![capture.session canAddInput:input]) {*error=strdup("This camera input could not be added. Check whether another app is using it.");return NULL;}
            [capture.session addInput:input];
            capture.output.videoSettings=@{(id)kCVPixelBufferPixelFormatTypeKey:@(kCVPixelFormatType_32BGRA)};
            capture.output.alwaysDiscardsLateVideoFrames=YES;
            [capture.output setSampleBufferDelegate:capture queue:capture.queue];
            if (![capture.session canAddOutput:capture.output]) {*error=strdup("This camera does not provide a supported video stream.");return NULL;}
            [capture.session addOutput:capture.output];[capture.session commitConfiguration];
            [capture.session startRunning];
            if (!capture.session.running) {*error=strdup("The camera session did not start. Check the USB connection and other apps using the camera.");return NULL;}
            return (void *)CFBridgingRetain(capture);
        } @catch (NSException *e) {*error=strdup((e.reason ?: @"Camera startup failed.").UTF8String);return NULL;}
    }
}
void commons_camera_stop(void *handle) {
    if (!handle) return;
    @autoreleasepool {
        CCCamera *capture=(__bridge CCCamera *)handle;
        atomic_store(&capture->active,false);
        [capture.output setSampleBufferDelegate:nil queue:NULL];
        [capture.session stopRunning];
        dispatch_sync(capture.queue,^{});
        [capture.session beginConfiguration];
        for (AVCaptureInput *input in [capture.session.inputs copy]) [capture.session removeInput:input];
        for (AVCaptureOutput *output in [capture.session.outputs copy]) [capture.session removeOutput:output];
        [capture.session commitConfiguration];
        CFRelease(handle);
    }
}
void commons_camera_free(char *text) {free(text);}
