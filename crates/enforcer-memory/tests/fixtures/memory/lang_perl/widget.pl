use strict;
use warnings;
use POSIX qw(floor);

package Widget;
require Exporter;

sub new {
    my $class = shift;
    my $self = { name => shift };
    bless $self, $class;
    return $self;
}

sub draw {
    my $self = shift;
    if ($self->{name}) {
        print("drawing: " . $self->{name});
    } else {
        print("drawing: unnamed");
    }
    return helper($self);
}

sub helper {
    my $widget = shift;
    return $widget->{name};
}

my $w = Widget->new("box");
$w->draw();
