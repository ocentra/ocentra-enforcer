#import <Foundation/Foundation.h>

@interface Animal : NSObject
- (void)speak;
@end

@interface Widget : Animal
- (void)draw;
- (void)setName:(NSString *)name withAge:(int)age;
@end

@implementation Widget
- (void)draw {
    [self speak];
    NSLog(@"drawing");
}

- (void)setName:(NSString *)name withAge:(int)age {
    [self helper:name];
}

- (void)helper:(NSString *)label {
    NSLog(@"%@", label);
}
@end
