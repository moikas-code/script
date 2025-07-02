# Game Development with Script

Welcome to game development with Script! This comprehensive guide will teach you how to create games using Script's powerful features designed specifically for game development. From simple 2D games to complex interactive experiences, Script provides the tools you need while maintaining beginner-friendly syntax.

## Table of Contents

1. [Why Script for Game Development?](#why-script-for-game-development)
2. [Setting Up Your Game Project](#setting-up-your-game-project)
3. [Core Game Concepts](#core-game-concepts)
4. [Math and Physics](#math-and-physics)
5. [Graphics and Rendering](#graphics-and-rendering)
6. [Input Handling](#input-handling)
7. [Game State Management](#game-state-management)
8. [Audio Systems](#audio-systems)
9. [Asset Management](#asset-management)
10. [Performance Optimization](#performance-optimization)
11. [Complete Game Examples](#complete-game-examples)
12. [Publishing and Distribution](#publishing-and-distribution)

## Why Script for Game Development?

Script is uniquely designed for game development with several key advantages:

### **Beginner-Friendly Yet Powerful**
```script
// Simple enough for beginners
fn update_player(player: Player, input: Input, dt: f32) {
    if input.is_pressed("left") {
        player.position.x -= player.speed * dt
    }
    if input.is_pressed("right") {
        player.position.x += player.speed * dt
    }
}

// Powerful enough for complex systems
async fn load_level(level_id: i32) -> Result<Level, GameError> {
    let level_data = await load_asset("levels/level" + level_id + ".json")?
    let parsed = parse_level_data(level_data)?
    let textures = await load_level_textures(parsed.texture_list)?
    
    Ok(Level::new(parsed, textures))
}
```

### **Performance Without Complexity**
- JIT compilation for hot code paths
- Zero-cost abstractions for high-level code
- Automatic memory management without GC pauses
- Built-in profiling and optimization tools

### **Built-in Game Development Features**
- Vector math (Vec2, Vec3, Vec4)
- Color manipulation (RGBA, HSV, HSL)
- Time and animation systems
- Random number generation
- Asset loading and management

## Setting Up Your Game Project

### Project Structure

```
my_game/
├── script.toml           # Project configuration
├── src/
│   ├── main.script      # Entry point
│   ├── game/            # Core game logic
│   │   ├── mod.script
│   │   ├── player.script
│   │   ├── enemies.script
│   │   └── level.script
│   ├── systems/         # Game systems
│   │   ├── mod.script
│   │   ├── physics.script
│   │   ├── rendering.script
│   │   └── audio.script
│   └── utils/           # Utility functions
│       ├── mod.script
│       └── math.script
├── assets/              # Game assets
│   ├── sprites/
│   ├── sounds/
│   ├── levels/
│   └── fonts/
└── README.md
```

### Basic Game Loop

```script
// main.script - Basic game structure
import game::GameState
import systems::*

const WINDOW_WIDTH: i32 = 800
const WINDOW_HEIGHT: i32 = 600
const TARGET_FPS: f32 = 60.0

fn main() {
    // Initialize game systems
    let window = Window::new(WINDOW_WIDTH, WINDOW_HEIGHT, "My Script Game")
    let renderer = Renderer::new(window)
    let input = Input::new()
    let audio = Audio::new()
    
    // Create game state
    let mut game_state = GameState::new()
    
    // Main game loop
    let mut last_time = time_now()
    let mut running = true
    
    while running {
        let current_time = time_now()
        let dt = current_time - last_time
        last_time = current_time
        
        // Handle events
        let events = window.poll_events()
        for event in events {
            match event {
                Event::Quit => running = false,
                Event::KeyPress(key) => input.handle_key_press(key),
                Event::KeyRelease(key) => input.handle_key_release(key),
                _ => {}
            }
        }
        
        // Update game logic
        game_state.update(input, dt)
        
        // Render frame
        renderer.clear(Color::BLACK)
        game_state.render(renderer)
        renderer.present()
        
        // Maintain target framerate
        let frame_time = time_now() - current_time
        let target_frame_time = 1.0 / TARGET_FPS
        if frame_time < target_frame_time {
            sleep((target_frame_time - frame_time) * 1000.0)
        }
    }
    
    // Cleanup
    audio.cleanup()
    renderer.cleanup()
    window.close()
}
```

## Core Game Concepts

### Game Objects and Components

```script
// Base game object system
struct GameObject {
    id: i32,
    position: Vec2,
    rotation: f32,
    scale: Vec2,
    active: bool,
    components: HashMap<string, Component>
}

trait Component {
    fn update(mut self, dt: f32)
    fn render(self, renderer: Renderer)
}

impl GameObject {
    fn new(id: i32) -> GameObject {
        GameObject {
            id,
            position: vec2(0.0, 0.0),
            rotation: 0.0,
            scale: vec2(1.0, 1.0),
            active: true,
            components: HashMap::new()
        }
    }
    
    fn add_component<T: Component>(mut self, name: string, component: T) {
        hashmap_insert(self.components, name, component)
    }
    
    fn get_component<T: Component>(self, name: string) -> Option<T> {
        hashmap_get(self.components, name)
    }
    
    fn update(mut self, dt: f32) {
        if !self.active { return }
        
        // Update all components
        for (name, component) in self.components {
            component.update(dt)
        }
    }
    
    fn render(self, renderer: Renderer) {
        if !self.active { return }
        
        // Render all components
        for (name, component) in self.components {
            component.render(renderer)
        }
    }
}

// Example components
struct SpriteComponent {
    texture: Texture,
    source_rect: Rect,
    color: Color
}

impl Component for SpriteComponent {
    fn update(mut self, dt: f32) {
        // No update needed for static sprites
    }
    
    fn render(self, renderer: Renderer) {
        renderer.draw_texture(
            self.texture,
            self.source_rect,
            self.color
        )
    }
}

struct PhysicsComponent {
    velocity: Vec2,
    acceleration: Vec2,
    mass: f32,
    drag: f32
}

impl Component for PhysicsComponent {
    fn update(mut self, dt: f32) {
        // Apply acceleration to velocity
        self.velocity = vec2_add(self.velocity, vec2_scale(self.acceleration, dt))
        
        // Apply drag
        self.velocity = vec2_scale(self.velocity, 1.0 - self.drag * dt)
        
        // Update position (this would need access to the parent GameObject)
        // This is simplified - in practice you'd have a reference system
    }
    
    fn render(self, renderer: Renderer) {
        // Physics components don't render
    }
}
```

### Scene Management

```script
// Scene system for organizing game states
trait Scene {
    fn enter(mut self)
    fn exit(mut self)
    fn update(mut self, input: Input, dt: f32) -> Option<SceneTransition>
    fn render(self, renderer: Renderer)
}

enum SceneTransition {
    None,
    Push(Scene),
    Pop,
    Replace(Scene),
    Clear
}

struct SceneManager {
    scenes: Vec<Scene>,
    pending_transition: Option<SceneTransition>
}

impl SceneManager {
    fn new() -> SceneManager {
        SceneManager {
            scenes: Vec::new(),
            pending_transition: None
        }
    }
    
    fn push(mut self, scene: Scene) {
        scene.enter()
        vec_push(self.scenes, scene)
    }
    
    fn update(mut self, input: Input, dt: f32) {
        if let Some(current_scene) = vec_last_mut(self.scenes) {
            let transition = current_scene.update(input, dt)
            self.pending_transition = transition
        }
        
        self.handle_transitions()
    }
    
    fn render(self, renderer: Renderer) {
        // Render all scenes (for overlay effects)
        for scene in self.scenes {
            scene.render(renderer)
        }
    }
    
    fn handle_transitions(mut self) {
        match self.pending_transition.take() {
            Some(SceneTransition::Push(scene)) => {
                self.push(scene)
            },
            Some(SceneTransition::Pop) => {
                if let Some(scene) = vec_pop(self.scenes) {
                    scene.exit()
                }
            },
            Some(SceneTransition::Replace(new_scene)) => {
                if let Some(old_scene) = vec_pop(self.scenes) {
                    old_scene.exit()
                }
                self.push(new_scene)
            },
            Some(SceneTransition::Clear) => {
                while let Some(scene) = vec_pop(self.scenes) {
                    scene.exit()
                }
            },
            None => {}
        }
    }
}

// Example scenes
struct MenuScene {
    selected_option: i32,
    options: Vec<string>
}

impl Scene for MenuScene {
    fn enter(mut self) {
        print("Entering menu scene")
        self.selected_option = 0
    }
    
    fn exit(mut self) {
        print("Exiting menu scene")
    }
    
    fn update(mut self, input: Input, dt: f32) -> Option<SceneTransition> {
        if input.was_pressed("up") {
            self.selected_option = max(0, self.selected_option - 1)
        }
        if input.was_pressed("down") {
            self.selected_option = min(vec_len(self.options) - 1, self.selected_option + 1)
        }
        if input.was_pressed("enter") {
            match self.selected_option {
                0 => return Some(SceneTransition::Push(GameScene::new())),
                1 => return Some(SceneTransition::Push(OptionsScene::new())),
                2 => return Some(SceneTransition::Clear),  // Quit
                _ => {}
            }
        }
        
        None
    }
    
    fn render(self, renderer: Renderer) {
        renderer.clear(Color::DARK_BLUE)
        
        let title_pos = vec2(WINDOW_WIDTH / 2, 100)
        renderer.draw_text("MY AWESOME GAME", title_pos, Color::WHITE, 32)
        
        for i in 0..vec_len(self.options) {
            let option = vec_get(self.options, i).unwrap()
            let y = 200 + i * 50
            let color = if i == self.selected_option { Color::YELLOW } else { Color::WHITE }
            
            renderer.draw_text(option, vec2(WINDOW_WIDTH / 2, y), color, 24)
        }
    }
}
```

## Math and Physics

Script provides built-in math utilities perfect for game development:

### Vector Math

```script
// 2D Vector operations
fn player_movement_example(player: Player, input: Input, dt: f32) {
    let mut direction = vec2(0.0, 0.0)
    
    if input.is_pressed("left") { direction.x -= 1.0 }
    if input.is_pressed("right") { direction.x += 1.0 }
    if input.is_pressed("up") { direction.y -= 1.0 }
    if input.is_pressed("down") { direction.y += 1.0 }
    
    // Normalize direction for consistent diagonal movement
    if vec2_length(direction) > 0.0 {
        direction = vec2_normalize(direction)
    }
    
    // Apply movement
    let velocity = vec2_scale(direction, player.speed)
    player.position = vec2_add(player.position, vec2_scale(velocity, dt))
}

// 3D Vector operations for 3D games
fn calculate_camera_position(target: Vec3, distance: f32, pitch: f32, yaw: f32) -> Vec3 {
    let x = distance * cos(pitch) * sin(yaw)
    let y = distance * sin(pitch)
    let z = distance * cos(pitch) * cos(yaw)
    
    vec3_add(target, vec3(x, y, z))
}

// Interpolation for smooth animations
fn smooth_follow_camera(camera: Camera, target: Vec2, dt: f32) {
    let follow_speed = 2.0
    camera.position = vec2_lerp(camera.position, target, follow_speed * dt)
}

// Distance and collision detection
fn check_circular_collision(pos1: Vec2, radius1: f32, pos2: Vec2, radius2: f32) -> bool {
    let distance = vec2_distance(pos1, pos2)
    distance < (radius1 + radius2)
}

fn check_rect_collision(rect1: Rect, rect2: Rect) -> bool {
    rect1.x < rect2.x + rect2.width &&
    rect1.x + rect1.width > rect2.x &&
    rect1.y < rect2.y + rect2.height &&
    rect1.y + rect1.height > rect2.y
}
```

### Advanced Math for Games

```script
// Easing functions for animations
fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let f = (2.0 * t) - 2.0
        1.0 + f * f * f / 2.0
    }
}

fn ease_out_bounce(t: f32) -> f32 {
    if t < (1.0 / 2.75) {
        7.5625 * t * t
    } else if t < (2.0 / 2.75) {
        let t2 = t - (1.5 / 2.75)
        7.5625 * t2 * t2 + 0.75
    } else if t < (2.5 / 2.75) {
        let t2 = t - (2.25 / 2.75)
        7.5625 * t2 * t2 + 0.9375
    } else {
        let t2 = t - (2.625 / 2.75)
        7.5625 * t2 * t2 + 0.984375
    }
}

// Animation system using easing
struct Animation {
    start_value: f32,
    end_value: f32,
    duration: f32,
    elapsed: f32,
    easing_function: fn(f32) -> f32,
    complete: bool
}

impl Animation {
    fn new(start: f32, end: f32, duration: f32, easing: fn(f32) -> f32) -> Animation {
        Animation {
            start_value: start,
            end_value: end,
            duration,
            elapsed: 0.0,
            easing_function: easing,
            complete: false
        }
    }
    
    fn update(mut self, dt: f32) -> f32 {
        if self.complete {
            return self.end_value
        }
        
        self.elapsed += dt
        if self.elapsed >= self.duration {
            self.elapsed = self.duration
            self.complete = true
        }
        
        let t = self.elapsed / self.duration
        let eased_t = (self.easing_function)(t)
        
        lerp(self.start_value, self.end_value, eased_t)
    }
    
    fn is_complete(self) -> bool {
        self.complete
    }
}

// Physics simulation
struct RigidBody {
    position: Vec2,
    velocity: Vec2,
    acceleration: Vec2,
    mass: f32,
    restitution: f32,  // Bounciness
    friction: f32
}

impl RigidBody {
    fn apply_force(mut self, force: Vec2) {
        let accel = vec2_scale(force, 1.0 / self.mass)
        self.acceleration = vec2_add(self.acceleration, accel)
    }
    
    fn apply_impulse(mut self, impulse: Vec2) {
        let velocity_change = vec2_scale(impulse, 1.0 / self.mass)
        self.velocity = vec2_add(self.velocity, velocity_change)
    }
    
    fn update(mut self, dt: f32) {
        // Apply gravity
        self.apply_force(vec2(0.0, 981.0))  // 9.81 m/s² scaled up
        
        // Update velocity from acceleration
        self.velocity = vec2_add(self.velocity, vec2_scale(self.acceleration, dt))
        
        // Apply friction
        let friction_force = vec2_scale(self.velocity, -self.friction)
        self.velocity = vec2_add(self.velocity, vec2_scale(friction_force, dt))
        
        // Update position from velocity
        self.position = vec2_add(self.position, vec2_scale(self.velocity, dt))
        
        // Reset acceleration for next frame
        self.acceleration = vec2(0.0, 0.0)
    }
    
    fn handle_collision(mut self, normal: Vec2) {
        // Reflect velocity based on surface normal and restitution
        let dot_product = vec2_dot(self.velocity, normal)
        let reflection = vec2_scale(normal, 2.0 * dot_product)
        self.velocity = vec2_scale(vec2_sub(self.velocity, reflection), self.restitution)
    }
}
```

## Graphics and Rendering

### Basic Rendering System

```script
// Color operations
struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32
}

impl Color {
    const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }
    const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }
    const RED: Color = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }
    const GREEN: Color = Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 }
    const BLUE: Color = Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 }
    
    fn new(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }
    
    fn from_rgb(r: i32, g: i32, b: i32) -> Color {
        Color {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0
        }
    }
    
    fn lerp(self, other: Color, t: f32) -> Color {
        Color {
            r: lerp(self.r, other.r, t),
            g: lerp(self.g, other.g, t),
            b: lerp(self.b, other.b, t),
            a: lerp(self.a, other.a, t)
        }
    }
}

// Basic renderer interface
trait Renderer {
    fn clear(mut self, color: Color)
    fn present(mut self)
    fn draw_rect(mut self, rect: Rect, color: Color)
    fn draw_circle(mut self, center: Vec2, radius: f32, color: Color)
    fn draw_line(mut self, start: Vec2, end: Vec2, color: Color, thickness: f32)
    fn draw_texture(mut self, texture: Texture, position: Vec2, color: Color)
    fn draw_text(mut self, text: string, position: Vec2, color: Color, size: f32)
}

// Sprite system
struct Sprite {
    texture: Texture,
    source_rect: Rect,
    position: Vec2,
    origin: Vec2,
    scale: Vec2,
    rotation: f32,
    color: Color
}

impl Sprite {
    fn new(texture: Texture) -> Sprite {
        let texture_size = texture.get_size()
        Sprite {
            texture,
            source_rect: rect(0, 0, texture_size.x, texture_size.y),
            position: vec2(0.0, 0.0),
            origin: vec2(texture_size.x / 2.0, texture_size.y / 2.0),
            scale: vec2(1.0, 1.0),
            rotation: 0.0,
            color: Color::WHITE
        }
    }
    
    fn draw(self, renderer: Renderer) {
        renderer.draw_sprite(
            self.texture,
            self.source_rect,
            self.position,
            self.origin,
            self.scale,
            self.rotation,
            self.color
        )
    }
}

// Animation system
struct AnimatedSprite {
    sprite: Sprite,
    frames: Vec<Rect>,
    current_frame: i32,
    frame_time: f32,
    elapsed_time: f32,
    loop_animation: bool
}

impl AnimatedSprite {
    fn new(texture: Texture, frame_width: i32, frame_height: i32) -> AnimatedSprite {
        let texture_size = texture.get_size()
        let mut frames = Vec::new()
        
        // Generate frames from sprite sheet
        let cols = texture_size.x / frame_width
        let rows = texture_size.y / frame_height
        
        for row in 0..rows {
            for col in 0..cols {
                let frame = rect(
                    col * frame_width,
                    row * frame_height,
                    frame_width,
                    frame_height
                )
                vec_push(frames, frame)
            }
        }
        
        AnimatedSprite {
            sprite: Sprite::new(texture),
            frames,
            current_frame: 0,
            frame_time: 0.1,  // 10 FPS default
            elapsed_time: 0.0,
            loop_animation: true
        }
    }
    
    fn update(mut self, dt: f32) {
        self.elapsed_time += dt
        
        if self.elapsed_time >= self.frame_time {
            self.elapsed_time = 0.0
            self.current_frame += 1
            
            if self.current_frame >= vec_len(self.frames) {
                if self.loop_animation {
                    self.current_frame = 0
                } else {
                    self.current_frame = vec_len(self.frames) - 1
                }
            }
        }
        
        // Update sprite's source rectangle
        if let Some(frame) = vec_get(self.frames, self.current_frame) {
            self.sprite.source_rect = frame
        }
    }
    
    fn draw(self, renderer: Renderer) {
        self.sprite.draw(renderer)
    }
}
```

### Particle System

```script
// Particle system for effects
struct Particle {
    position: Vec2,
    velocity: Vec2,
    acceleration: Vec2,
    life: f32,
    max_life: f32,
    color: Color,
    size: f32,
    rotation: f32,
    angular_velocity: f32
}

impl Particle {
    fn new(position: Vec2, velocity: Vec2, life: f32, color: Color, size: f32) -> Particle {
        Particle {
            position,
            velocity,
            acceleration: vec2(0.0, 0.0),
            life,
            max_life: life,
            color,
            size,
            rotation: 0.0,
            angular_velocity: 0.0
        }
    }
    
    fn update(mut self, dt: f32) -> bool {
        // Update physics
        self.velocity = vec2_add(self.velocity, vec2_scale(self.acceleration, dt))
        self.position = vec2_add(self.position, vec2_scale(self.velocity, dt))
        self.rotation += self.angular_velocity * dt
        
        // Update life
        self.life -= dt
        
        // Fade out over time
        let life_ratio = self.life / self.max_life
        self.color.a = life_ratio
        
        self.life > 0.0
    }
    
    fn draw(self, renderer: Renderer) {
        renderer.draw_circle(self.position, self.size, self.color)
    }
}

struct ParticleSystem {
    particles: Vec<Particle>,
    emission_rate: f32,
    emission_timer: f32,
    position: Vec2,
    max_particles: i32
}

impl ParticleSystem {
    fn new(position: Vec2, emission_rate: f32, max_particles: i32) -> ParticleSystem {
        ParticleSystem {
            particles: Vec::new(),
            emission_rate,
            emission_timer: 0.0,
            position,
            max_particles
        }
    }
    
    fn emit_particle(mut self) {
        if vec_len(self.particles) >= self.max_particles {
            return
        }
        
        // Random particle properties
        let angle = random() * 2.0 * 3.14159
        let speed = random_range(50.0, 150.0)
        let velocity = vec2(cos(angle) * speed, sin(angle) * speed)
        
        let life = random_range(1.0, 3.0)
        let size = random_range(2.0, 8.0)
        
        let hue = random() * 360.0
        let color = Color::from_hsv(hue, 1.0, 1.0)
        
        let particle = Particle::new(self.position, velocity, life, color, size)
        vec_push(self.particles, particle)
    }
    
    fn update(mut self, dt: f32) {
        // Emit new particles
        self.emission_timer += dt
        if self.emission_timer >= (1.0 / self.emission_rate) {
            self.emission_timer = 0.0
            self.emit_particle()
        }
        
        // Update existing particles
        let mut i = 0
        while i < vec_len(self.particles) {
            match vec_get_mut(self.particles, i) {
                Some(particle) => {
                    if particle.update(dt) {
                        i += 1
                    } else {
                        vec_remove(self.particles, i)
                    }
                },
                None => break
            }
        }
    }
    
    fn draw(self, renderer: Renderer) {
        for particle in self.particles {
            particle.draw(renderer)
        }
    }
}
```

## Input Handling

### Input Management System

```script
// Input state tracking
struct InputState {
    keys_pressed: HashMap<string, bool>,
    keys_just_pressed: HashMap<string, bool>,
    keys_just_released: HashMap<string, bool>,
    mouse_position: Vec2,
    mouse_buttons: HashMap<string, bool>,
    mouse_wheel: f32
}

impl InputState {
    fn new() -> InputState {
        InputState {
            keys_pressed: HashMap::new(),
            keys_just_pressed: HashMap::new(),
            keys_just_released: HashMap::new(),
            mouse_position: vec2(0.0, 0.0),
            mouse_buttons: HashMap::new(),
            mouse_wheel: 0.0
        }
    }
    
    fn update(mut self) {
        // Clear just pressed/released states
        self.keys_just_pressed.clear()
        self.keys_just_released.clear()
        self.mouse_wheel = 0.0
    }
    
    fn handle_key_press(mut self, key: string) {
        if !self.is_pressed(&key) {
            hashmap_insert(self.keys_just_pressed, key.clone(), true)
        }
        hashmap_insert(self.keys_pressed, key, true)
    }
    
    fn handle_key_release(mut self, key: string) {
        hashmap_insert(self.keys_just_released, key.clone(), true)
        hashmap_remove(self.keys_pressed, &key)
    }
    
    fn is_pressed(self, key: &string) -> bool {
        hashmap_get(self.keys_pressed, key).unwrap_or(false)
    }
    
    fn was_just_pressed(self, key: &string) -> bool {
        hashmap_get(self.keys_just_pressed, key).unwrap_or(false)
    }
    
    fn was_just_released(self, key: &string) -> bool {
        hashmap_get(self.keys_just_released, key).unwrap_or(false)
    }
}

// Input mapping system
struct InputMap {
    key_bindings: HashMap<string, string>,
    gamepad_bindings: HashMap<string, string>
}

impl InputMap {
    fn new() -> InputMap {
        let mut input_map = InputMap {
            key_bindings: HashMap::new(),
            gamepad_bindings: HashMap::new()
        }
        
        // Default bindings
        input_map.bind_key("move_left", "A")
        input_map.bind_key("move_right", "D")
        input_map.bind_key("move_up", "W")
        input_map.bind_key("move_down", "S")
        input_map.bind_key("jump", "SPACE")
        input_map.bind_key("attack", "J")
        input_map.bind_key("pause", "ESCAPE")
        
        input_map
    }
    
    fn bind_key(mut self, action: string, key: string) {
        hashmap_insert(self.key_bindings, action, key)
    }
    
    fn get_action_state(self, action: string, input: InputState) -> bool {
        match hashmap_get(self.key_bindings, &action) {
            Some(key) => input.is_pressed(key),
            None => false
        }
    }
    
    fn was_action_just_pressed(self, action: string, input: InputState) -> bool {
        match hashmap_get(self.key_bindings, &action) {
            Some(key) => input.was_just_pressed(key),
            None => false
        }
    }
}

// Player controller example
struct PlayerController {
    input_map: InputMap,
    move_speed: f32,
    jump_force: f32
}

impl PlayerController {
    fn new() -> PlayerController {
        PlayerController {
            input_map: InputMap::new(),
            move_speed: 200.0,
            jump_force: 400.0
        }
    }
    
    fn update(self, player: Player, input: InputState, dt: f32) {
        let mut movement = vec2(0.0, 0.0)
        
        // Horizontal movement
        if self.input_map.get_action_state("move_left", input) {
            movement.x -= 1.0
        }
        if self.input_map.get_action_state("move_right", input) {
            movement.x += 1.0
        }
        
        // Apply movement
        if vec2_length(movement) > 0.0 {
            movement = vec2_normalize(movement)
            let velocity = vec2_scale(movement, self.move_speed)
            player.physics.velocity.x = velocity.x
        }
        
        // Jumping
        if self.input_map.was_action_just_pressed("jump", input) && player.is_grounded {
            player.physics.velocity.y = -self.jump_force
            player.is_grounded = false
        }
        
        // Combat
        if self.input_map.was_action_just_pressed("attack", input) {
            player.attack()
        }
    }
}
```

## Game State Management

### State Machine Implementation

```script
// Game state enumeration
enum GameStateType {
    MainMenu,
    Playing,
    Paused,
    GameOver,
    LevelComplete,
    Settings
}

// State machine for game flow
struct GameStateMachine {
    current_state: GameStateType,
    states: HashMap<GameStateType, GameState>,
    state_stack: Vec<GameStateType>
}

impl GameStateMachine {
    fn new() -> GameStateMachine {
        let mut machine = GameStateMachine {
            current_state: GameStateType::MainMenu,
            states: HashMap::new(),
            state_stack: Vec::new()
        }
        
        // Register states
        hashmap_insert(machine.states, GameStateType::MainMenu, MainMenuState::new())
        hashmap_insert(machine.states, GameStateType::Playing, PlayingState::new())
        hashmap_insert(machine.states, GameStateType::Paused, PausedState::new())
        hashmap_insert(machine.states, GameStateType::GameOver, GameOverState::new())
        
        machine
    }
    
    fn change_state(mut self, new_state: GameStateType) {
        // Exit current state
        if let Some(current) = hashmap_get_mut(self.states, &self.current_state) {
            current.exit()
        }
        
        self.current_state = new_state
        
        // Enter new state
        if let Some(new) = hashmap_get_mut(self.states, &self.current_state) {
            new.enter()
        }
    }
    
    fn push_state(mut self, state: GameStateType) {
        vec_push(self.state_stack, self.current_state)
        self.change_state(state)
    }
    
    fn pop_state(mut self) {
        if let Some(previous_state) = vec_pop(self.state_stack) {
            self.change_state(previous_state)
        }
    }
    
    fn update(mut self, input: InputState, dt: f32) {
        if let Some(current) = hashmap_get_mut(self.states, &self.current_state) {
            current.update(self, input, dt)
        }
    }
    
    fn render(self, renderer: Renderer) {
        if let Some(current) = hashmap_get(self.states, &self.current_state) {
            current.render(renderer)
        }
    }
}

// Playing state implementation
struct PlayingState {
    level: Level,
    player: Player,
    enemies: Vec<Enemy>,
    ui: GameUI,
    camera: Camera
}

impl GameState for PlayingState {
    fn enter(mut self) {
        print("Entering playing state")
        self.camera.follow_target(self.player.position)
    }
    
    fn exit(mut self) {
        print("Exiting playing state")
    }
    
    fn update(mut self, state_machine: GameStateMachine, input: InputState, dt: f32) {
        // Check for pause
        if input.was_just_pressed("ESCAPE") {
            state_machine.push_state(GameStateType::Paused)
            return
        }
        
        // Update game objects
        self.player.update(input, dt)
        
        for enemy in self.enemies {
            enemy.update(self.player.position, dt)
        }
        
        self.level.update(dt)
        self.camera.follow_target(self.player.position)
        
        // Check win/lose conditions
        if self.player.health <= 0 {
            state_machine.change_state(GameStateType::GameOver)
        } else if self.level.is_complete() {
            state_machine.change_state(GameStateType::LevelComplete)
        }
    }
    
    fn render(self, renderer: Renderer) {
        // Set camera transform
        renderer.set_camera(self.camera)
        
        // Render world
        self.level.render(renderer)
        self.player.render(renderer)
        
        for enemy in self.enemies {
            enemy.render(renderer)
        }
        
        // Reset camera for UI
        renderer.reset_camera()
        
        // Render UI
        self.ui.render(renderer, self.player)
    }
}
```

### Save System

```script
// Save data structure
struct SaveData {
    player_name: string,
    current_level: i32,
    score: i32,
    high_score: i32,
    unlocked_levels: Vec<i32>,
    settings: GameSettings,
    achievements: Vec<string>,
    play_time: f64
}

impl SaveData {
    fn new() -> SaveData {
        SaveData {
            player_name: "Player".to_string(),
            current_level: 1,
            score: 0,
            high_score: 0,
            unlocked_levels: vec![1],
            settings: GameSettings::default(),
            achievements: Vec::new(),
            play_time: 0.0
        }
    }
    
    fn serialize(self) -> string {
        // Simple JSON-like serialization
        // In practice, you'd use a proper JSON library
        let mut json = "{\n"
        json += "  \"player_name\": \"" + self.player_name + "\",\n"
        json += "  \"current_level\": " + self.current_level.to_string() + ",\n"
        json += "  \"score\": " + self.score.to_string() + ",\n"
        json += "  \"high_score\": " + self.high_score.to_string() + ",\n"
        json += "  \"play_time\": " + self.play_time.to_string() + "\n"
        json += "}"
        json
    }
    
    fn deserialize(json: string) -> Result<SaveData, string> {
        // Simple JSON parsing - in practice use a proper parser
        let mut save_data = SaveData::new()
        
        // This is a simplified parser - real implementation would be more robust
        if contains(json, "player_name") {
            // Extract player name...
        }
        if contains(json, "current_level") {
            // Extract current level...
        }
        
        Result::ok(save_data)
    }
}

// Save system manager
struct SaveSystem {
    save_file_path: string,
    current_save: SaveData,
    auto_save_interval: f32,
    auto_save_timer: f32
}

impl SaveSystem {
    fn new(save_file: string) -> SaveSystem {
        SaveSystem {
            save_file_path: save_file,
            current_save: SaveData::new(),
            auto_save_interval: 30.0,  // Auto-save every 30 seconds
            auto_save_timer: 0.0
        }
    }
    
    fn load(mut self) -> Result<(), string> {
        match read_file(self.save_file_path) {
            Ok(content) => {
                match SaveData::deserialize(content) {
                    Ok(save_data) => {
                        self.current_save = save_data
                        Result::ok(())
                    },
                    Err(error) => Result::err("Failed to parse save file: " + error)
                }
            },
            Err(error) => {
                print("No save file found, creating new save")
                self.save()
            }
        }
    }
    
    fn save(self) -> Result<(), string> {
        let serialized = self.current_save.serialize()
        write_file(self.save_file_path, serialized)
    }
    
    fn update(mut self, dt: f32) {
        self.auto_save_timer += dt
        if self.auto_save_timer >= self.auto_save_interval {
            self.auto_save_timer = 0.0
            match self.save() {
                Ok(()) => print("Auto-saved game"),
                Err(error) => print("Auto-save failed: " + error)
            }
        }
    }
    
    fn unlock_level(mut self, level: i32) {
        if !vec_contains(self.current_save.unlocked_levels, level) {
            vec_push(self.current_save.unlocked_levels, level)
        }
    }
    
    fn add_achievement(mut self, achievement: string) {
        if !vec_contains(self.current_save.achievements, achievement) {
            vec_push(self.current_save.achievements, achievement)
            print("Achievement unlocked: " + achievement)
        }
    }
}
```

## Audio Systems

### Audio Management

```script
// Audio types and management
enum AudioType {
    Music,
    SoundEffect,
    Voice,
    Ambient
}

struct AudioClip {
    data: AudioData,
    volume: f32,
    loop_audio: bool,
    audio_type: AudioType
}

struct AudioManager {
    music_volume: f32,
    sfx_volume: f32,
    master_volume: f32,
    current_music: Option<AudioClip>,
    playing_sounds: Vec<PlayingSound>
}

impl AudioManager {
    fn new() -> AudioManager {
        AudioManager {
            music_volume: 0.7,
            sfx_volume: 0.8,
            master_volume: 1.0,
            current_music: None,
            playing_sounds: Vec::new()
        }
    }
    
    fn play_music(mut self, music: AudioClip) {
        // Stop current music
        if let Some(current) = self.current_music {
            self.stop_audio(current)
        }
        
        // Play new music
        music.volume = self.music_volume * self.master_volume
        self.current_music = Some(music)
        self.start_audio(music)
    }
    
    fn play_sound(mut self, sound: AudioClip) -> i32 {
        sound.volume = self.sfx_volume * self.master_volume
        let sound_id = self.start_audio(sound)
        
        let playing_sound = PlayingSound {
            id: sound_id,
            clip: sound,
            start_time: time_now()
        }
        vec_push(self.playing_sounds, playing_sound)
        
        sound_id
    }
    
    fn play_sound_at_position(mut self, sound: AudioClip, position: Vec2, listener_pos: Vec2) -> i32 {
        // Calculate 3D audio effects
        let distance = vec2_distance(position, listener_pos)
        let max_distance = 1000.0
        let distance_volume = clamp(1.0 - (distance / max_distance), 0.0, 1.0)
        
        sound.volume = self.sfx_volume * self.master_volume * distance_volume
        
        // Calculate stereo panning
        let pan = clamp((position.x - listener_pos.x) / 500.0, -1.0, 1.0)
        sound.pan = pan
        
        self.play_sound(sound)
    }
    
    fn update(mut self, dt: f32) {
        // Clean up finished sounds
        let mut i = 0
        while i < vec_len(self.playing_sounds) {
            match vec_get(self.playing_sounds, i) {
                Some(playing_sound) => {
                    if !self.is_playing(playing_sound.id) {
                        vec_remove(self.playing_sounds, i)
                    } else {
                        i += 1
                    }
                },
                None => break
            }
        }
    }
    
    fn set_master_volume(mut self, volume: f32) {
        self.master_volume = clamp(volume, 0.0, 1.0)
        self.update_all_volumes()
    }
    
    fn set_music_volume(mut self, volume: f32) {
        self.music_volume = clamp(volume, 0.0, 1.0)
        if let Some(music) = self.current_music {
            music.volume = self.music_volume * self.master_volume
        }
    }
    
    fn set_sfx_volume(mut self, volume: f32) {
        self.sfx_volume = clamp(volume, 0.0, 1.0)
    }
}

// Dynamic music system
struct MusicManager {
    audio_manager: AudioManager,
    current_track: Option<string>,
    fade_duration: f32,
    crossfade_active: bool,
    fade_timer: f32
}

impl MusicManager {
    fn new(audio_manager: AudioManager) -> MusicManager {
        MusicManager {
            audio_manager,
            current_track: None,
            fade_duration: 2.0,
            crossfade_active: false,
            fade_timer: 0.0
        }
    }
    
    fn play_track(mut self, track_name: string, fade_in: bool) {
        if fade_in && self.current_track.is_some() {
            self.crossfade_to_track(track_name)
        } else {
            let music = load_audio("music/" + track_name + ".ogg")
            self.audio_manager.play_music(music)
            self.current_track = Some(track_name)
        }
    }
    
    fn crossfade_to_track(mut self, new_track: string) {
        self.crossfade_active = true
        self.fade_timer = 0.0
        // Implementation would handle gradual volume changes
    }
    
    fn update(mut self, dt: f32) {
        if self.crossfade_active {
            self.fade_timer += dt
            let fade_progress = self.fade_timer / self.fade_duration
            
            if fade_progress >= 1.0 {
                self.crossfade_active = false
                // Complete the crossfade
            } else {
                // Update volumes for crossfade effect
                let old_volume = 1.0 - fade_progress
                let new_volume = fade_progress
                // Apply volumes...
            }
        }
        
        self.audio_manager.update(dt)
    }
}
```

## Complete Game Examples

### Simple Platformer Game

```script
// A complete simple platformer game
struct PlatformerGame {
    player: Player,
    platforms: Vec<Platform>,
    enemies: Vec<Enemy>,
    collectibles: Vec<Coin>,
    camera: Camera,
    ui: GameUI,
    level: i32,
    score: i32
}

impl PlatformerGame {
    fn new() -> PlatformerGame {
        let mut game = PlatformerGame {
            player: Player::new(vec2(100.0, 300.0)),
            platforms: Vec::new(),
            enemies: Vec::new(),
            collectibles: Vec::new(),
            camera: Camera::new(),
            ui: GameUI::new(),
            level: 1,
            score: 0
        }
        
        game.load_level(1)
        game
    }
    
    fn load_level(mut self, level_num: i32) {
        // Clear existing objects
        self.platforms.clear()
        self.enemies.clear()
        self.collectibles.clear()
        
        match level_num {
            1 => self.load_level_1(),
            2 => self.load_level_2(),
            _ => self.load_level_1()
        }
    }
    
    fn load_level_1(mut self) {
        // Ground platforms
        vec_push(self.platforms, Platform::new(rect(0, 400, 800, 50)))
        vec_push(self.platforms, Platform::new(rect(200, 300, 150, 20)))
        vec_push(self.platforms, Platform::new(rect(400, 250, 150, 20)))
        
        // Enemies
        vec_push(self.enemies, Enemy::new(vec2(300.0, 380.0), EnemyType::Walker))
        vec_push(self.enemies, Enemy::new(vec2(500.0, 230.0), EnemyType::Jumper))
        
        // Collectibles
        for i in 0..5 {
            let x = 150.0 + i as f32 * 100.0
            vec_push(self.collectibles, Coin::new(vec2(x, 350.0)))
        }
    }
    
    fn update(mut self, input: InputState, dt: f32) {
        // Update player
        self.player.update(input, dt)
        
        // Check platform collisions
        for platform in self.platforms {
            if platform.check_collision(self.player.get_bounds()) {
                self.player.handle_platform_collision(platform)
            }
        }
        
        // Update enemies
        for enemy in self.enemies {
            enemy.update(dt)
            
            // Check enemy-player collision
            if enemy.check_collision(self.player.get_bounds()) {
                if self.player.velocity.y > 0 && self.player.position.y < enemy.position.y {
                    // Player jumped on enemy
                    enemy.destroy()
                    self.player.bounce()
                    self.score += 100
                } else {
                    // Player hit enemy
                    self.player.take_damage()
                }
            }
        }
        
        // Check collectible collection
        let mut i = 0
        while i < vec_len(self.collectibles) {
            match vec_get(self.collectibles, i) {
                Some(coin) => {
                    if coin.check_collection(self.player.get_bounds()) {
                        self.score += 50
                        vec_remove(self.collectibles, i)
                        // Play collection sound
                    } else {
                        i += 1
                    }
                },
                None => break
            }
        }
        
        // Update camera to follow player
        self.camera.follow_target(self.player.position, dt)
        
        // Check level completion
        if vec_len(self.collectibles) == 0 {
            self.level += 1
            self.load_level(self.level)
        }
    }
    
    fn render(self, renderer: Renderer) {
        // Set camera view
        renderer.set_camera(self.camera)
        
        // Render platforms
        for platform in self.platforms {
            platform.render(renderer)
        }
        
        // Render enemies
        for enemy in self.enemies {
            enemy.render(renderer)
        }
        
        // Render collectibles
        for coin in self.collectibles {
            coin.render(renderer)
        }
        
        // Render player
        self.player.render(renderer)
        
        // Reset camera for UI
        renderer.reset_camera()
        
        // Render UI
        self.ui.render_score(renderer, self.score)
        self.ui.render_lives(renderer, self.player.lives)
        self.ui.render_level(renderer, self.level)
    }
}

// Player implementation
struct Player {
    position: Vec2,
    velocity: Vec2,
    size: Vec2,
    speed: f32,
    jump_force: f32,
    grounded: bool,
    lives: i32,
    animation: AnimatedSprite
}

impl Player {
    fn new(position: Vec2) -> Player {
        let texture = load_texture("player_sprite.png")
        Player {
            position,
            velocity: vec2(0.0, 0.0),
            size: vec2(32.0, 48.0),
            speed: 200.0,
            jump_force: 400.0,
            grounded: false,
            lives: 3,
            animation: AnimatedSprite::new(texture, 32, 48)
        }
    }
    
    fn update(mut self, input: InputState, dt: f32) {
        // Horizontal movement
        let mut movement = 0.0
        if input.is_pressed("A") || input.is_pressed("LEFT") {
            movement -= 1.0
        }
        if input.is_pressed("D") || input.is_pressed("RIGHT") {
            movement += 1.0
        }
        
        self.velocity.x = movement * self.speed
        
        // Jumping
        if (input.was_just_pressed("SPACE") || input.was_just_pressed("W")) && self.grounded {
            self.velocity.y = -self.jump_force
            self.grounded = false
        }
        
        // Apply gravity
        if !self.grounded {
            self.velocity.y += 981.0 * dt  // Gravity
        }
        
        // Update position
        self.position = vec2_add(self.position, vec2_scale(self.velocity, dt))
        
        // Update animation
        if abs(self.velocity.x) > 10.0 {
            self.animation.play("walk")
        } else {
            self.animation.play("idle")
        }
        
        if !self.grounded {
            self.animation.play("jump")
        }
        
        self.animation.update(dt)
    }
    
    fn render(self, renderer: Renderer) {
        self.animation.sprite.position = self.position
        self.animation.draw(renderer)
    }
    
    fn get_bounds(self) -> Rect {
        rect(self.position.x, self.position.y, self.size.x, self.size.y)
    }
    
    fn handle_platform_collision(mut self, platform: Platform) {
        let player_bounds = self.get_bounds()
        let platform_bounds = platform.get_bounds()
        
        // Simple collision response - land on top
        if self.velocity.y > 0 && player_bounds.bottom() <= platform_bounds.top() + 10 {
            self.position.y = platform_bounds.top() - self.size.y
            self.velocity.y = 0.0
            self.grounded = true
        }
    }
    
    fn take_damage(mut self) {
        self.lives -= 1
        // Add invincibility frames, knockback, etc.
    }
    
    fn bounce(mut self) {
        self.velocity.y = -200.0
    }
}
```

### Space Shooter Game

```script
// A complete space shooter game
struct SpaceShooterGame {
    player: Spaceship,
    enemies: Vec<Enemy>,
    bullets: Vec<Bullet>,
    enemy_bullets: Vec<Bullet>,
    powerups: Vec<PowerUp>,
    explosions: Vec<Explosion>,
    background: ScrollingBackground,
    score: i32,
    level: i32,
    enemy_spawn_timer: f32,
    audio: AudioManager
}

impl SpaceShooterGame {
    fn new() -> SpaceShooterGame {
        SpaceShooterGame {
            player: Spaceship::new(vec2(WINDOW_WIDTH / 2, WINDOW_HEIGHT - 100)),
            enemies: Vec::new(),
            bullets: Vec::new(),
            enemy_bullets: Vec::new(),
            powerups: Vec::new(),
            explosions: Vec::new(),
            background: ScrollingBackground::new(),
            score: 0,
            level: 1,
            enemy_spawn_timer: 0.0,
            audio: AudioManager::new()
        }
    }
    
    fn update(mut self, input: InputState, dt: f32) {
        // Update player
        self.player.update(input, dt)
        
        // Handle player shooting
        if input.is_pressed("SPACE") {
            if let Some(bullet) = self.player.try_shoot() {
                vec_push(self.bullets, bullet)
                self.audio.play_sound(load_audio("laser.wav"))
            }
        }
        
        // Spawn enemies
        self.enemy_spawn_timer += dt
        let spawn_rate = max(0.5, 2.0 - self.level as f32 * 0.1)
        if self.enemy_spawn_timer >= spawn_rate {
            self.enemy_spawn_timer = 0.0
            self.spawn_enemy()
        }
        
        // Update bullets
        self.update_bullets(dt)
        
        // Update enemies
        self.update_enemies(dt)
        
        // Update explosions
        self.update_explosions(dt)
        
        // Update background
        self.background.update(dt)
        
        // Check collisions
        self.check_collisions()
        
        // Clean up off-screen objects
        self.cleanup_objects()
    }
    
    fn spawn_enemy(mut self) {
        let x = random_range(50.0, WINDOW_WIDTH - 50.0)
        let enemy_type = if random() < 0.7 {
            EnemyType::Basic
        } else {
            EnemyType::Fighter
        }
        
        vec_push(self.enemies, Enemy::new(vec2(x, -50.0), enemy_type))
    }
    
    fn update_bullets(mut self, dt: f32) {
        // Update player bullets
        for bullet in self.bullets {
            bullet.update(dt)
        }
        
        // Update enemy bullets
        for bullet in self.enemy_bullets {
            bullet.update(dt)
        }
    }
    
    fn update_enemies(mut self, dt: f32) {
        for enemy in self.enemies {
            enemy.update(dt)
            
            // Enemy shooting
            if enemy.can_shoot() && random() < 0.01 {
                if let Some(bullet) = enemy.shoot() {
                    vec_push(self.enemy_bullets, bullet)
                }
            }
        }
    }
    
    fn check_collisions(mut self) {
        // Player bullets vs enemies
        let mut bullet_i = 0
        while bullet_i < vec_len(self.bullets) {
            let mut hit = false
            let bullet = vec_get(self.bullets, bullet_i).unwrap()
            
            let mut enemy_i = 0
            while enemy_i < vec_len(self.enemies) {
                let enemy = vec_get(self.enemies, enemy_i).unwrap()
                
                if bullet.check_collision(enemy.get_bounds()) {
                    // Hit!
                    self.create_explosion(enemy.position)
                    self.score += enemy.get_score_value()
                    
                    // Maybe drop powerup
                    if random() < 0.1 {
                        vec_push(self.powerups, PowerUp::new(enemy.position))
                    }
                    
                    vec_remove(self.enemies, enemy_i)
                    vec_remove(self.bullets, bullet_i)
                    hit = true
                    break
                } else {
                    enemy_i += 1
                }
            }
            
            if !hit {
                bullet_i += 1
            }
        }
        
        // Enemy bullets vs player
        let mut bullet_i = 0
        while bullet_i < vec_len(self.enemy_bullets) {
            let bullet = vec_get(self.enemy_bullets, bullet_i).unwrap()
            
            if bullet.check_collision(self.player.get_bounds()) {
                self.player.take_damage()
                self.create_explosion(self.player.position)
                vec_remove(self.enemy_bullets, bullet_i)
            } else {
                bullet_i += 1
            }
        }
        
        // Player vs powerups
        let mut powerup_i = 0
        while powerup_i < vec_len(self.powerups) {
            let powerup = vec_get(self.powerups, powerup_i).unwrap()
            
            if powerup.check_collision(self.player.get_bounds()) {
                self.player.apply_powerup(powerup.powerup_type)
                self.audio.play_sound(load_audio("powerup.wav"))
                vec_remove(self.powerups, powerup_i)
            } else {
                powerup_i += 1
            }
        }
    }
    
    fn create_explosion(mut self, position: Vec2) {
        vec_push(self.explosions, Explosion::new(position))
        self.audio.play_sound(load_audio("explosion.wav"))
    }
    
    fn render(self, renderer: Renderer) {
        // Render background
        self.background.render(renderer)
        
        // Render player
        self.player.render(renderer)
        
        // Render enemies
        for enemy in self.enemies {
            enemy.render(renderer)
        }
        
        // Render bullets
        for bullet in self.bullets {
            bullet.render(renderer)
        }
        
        for bullet in self.enemy_bullets {
            bullet.render(renderer)
        }
        
        // Render powerups
        for powerup in self.powerups {
            powerup.render(renderer)
        }
        
        // Render explosions
        for explosion in self.explosions {
            explosion.render(renderer)
        }
        
        // Render UI
        self.render_ui(renderer)
    }
    
    fn render_ui(self, renderer: Renderer) {
        // Score
        renderer.draw_text(
            "Score: " + self.score.to_string(),
            vec2(10, 10),
            Color::WHITE,
            24
        )
        
        // Lives
        renderer.draw_text(
            "Lives: " + self.player.lives.to_string(),
            vec2(10, 40),
            Color::WHITE,
            24
        )
        
        // Level
        renderer.draw_text(
            "Level: " + self.level.to_string(),
            vec2(10, 70),
            Color::WHITE,
            24
        )
    }
}
```

## Performance Optimization

### Profiling Your Game

```script
// Built-in profiling for performance optimization
struct GameProfiler {
    frame_times: Vec<f32>,
    update_times: Vec<f32>,
    render_times: Vec<f32>,
    current_frame: i32,
    enabled: bool
}

impl GameProfiler {
    fn new() -> GameProfiler {
        GameProfiler {
            frame_times: Vec::new(),
            update_times: Vec::new(),
            render_times: Vec::new(),
            current_frame: 0,
            enabled: true
        }
    }
    
    fn start_frame(mut self) {
        if !self.enabled { return }
        
        self.current_frame += 1
        self.frame_start_time = time_now()
    }
    
    fn end_frame(mut self) {
        if !self.enabled { return }
        
        let frame_time = time_now() - self.frame_start_time
        vec_push(self.frame_times, frame_time)
        
        // Keep only last 60 frames
        if vec_len(self.frame_times) > 60 {
            vec_remove(self.frame_times, 0)
        }
    }
    
    fn get_average_fps(self) -> f32 {
        if vec_len(self.frame_times) == 0 { return 0.0 }
        
        let mut total = 0.0
        for time in self.frame_times {
            total += time
        }
        
        let average_frame_time = total / vec_len(self.frame_times) as f32
        1.0 / average_frame_time
    }
    
    fn render_debug_info(self, renderer: Renderer) {
        if !self.enabled { return }
        
        let fps = self.get_average_fps()
        let fps_text = "FPS: " + fps.to_string()
        
        renderer.draw_text(fps_text, vec2(10, WINDOW_HEIGHT - 30), Color::GREEN, 16)
        
        // Render frame time graph
        self.render_frame_graph(renderer)
    }
    
    fn render_frame_graph(self, renderer: Renderer) {
        let graph_x = WINDOW_WIDTH - 200
        let graph_y = 10
        let graph_width = 180
        let graph_height = 60
        
        // Background
        renderer.draw_rect(
            rect(graph_x, graph_y, graph_width, graph_height),
            Color::new(0.0, 0.0, 0.0, 0.5)
        )
        
        // Frame time bars
        let bar_width = graph_width as f32 / vec_len(self.frame_times) as f32
        for i in 0..vec_len(self.frame_times) {
            let frame_time = vec_get(self.frame_times, i).unwrap()
            let normalized_height = clamp(frame_time * 60.0, 0.0, 1.0)  // Normalize to 60 FPS
            let bar_height = normalized_height * graph_height as f32
            
            let color = if frame_time > (1.0 / 30.0) {
                Color::RED  // Below 30 FPS
            } else if frame_time > (1.0 / 60.0) {
                Color::YELLOW  // Below 60 FPS
            } else {
                Color::GREEN  // Good performance
            }
            
            renderer.draw_rect(
                rect(
                    graph_x + i * bar_width,
                    graph_y + graph_height - bar_height,
                    bar_width - 1,
                    bar_height
                ),
                color
            )
        }
    }
}

// Object pooling for performance
struct ObjectPool<T> {
    available: Vec<T>,
    in_use: Vec<T>,
    factory: fn() -> T
}

impl<T> ObjectPool<T> {
    fn new(factory: fn() -> T, initial_size: i32) -> ObjectPool<T> {
        let mut pool = ObjectPool {
            available: Vec::new(),
            in_use: Vec::new(),
            factory
        }
        
        // Pre-allocate objects
        for _ in 0..initial_size {
            vec_push(pool.available, factory())
        }
        
        pool
    }
    
    fn get(mut self) -> Option<T> {
        match vec_pop(self.available) {
            Some(obj) => {
                vec_push(self.in_use, obj)
                Some(obj)
            },
            None => {
                // Pool exhausted, create new object
                let obj = (self.factory)()
                vec_push(self.in_use, obj)
                Some(obj)
            }
        }
    }
    
    fn release(mut self, obj: T) {
        // Find and remove from in_use
        for i in 0..vec_len(self.in_use) {
            if vec_get(self.in_use, i) == Some(&obj) {
                vec_remove(self.in_use, i)
                break
            }
        }
        
        // Reset object state if needed
        obj.reset()
        
        // Return to available pool
        vec_push(self.available, obj)
    }
}

// Usage example with bullets
fn create_optimized_shooter() -> SpaceShooterGame {
    let mut game = SpaceShooterGame::new()
    
    // Create bullet pool
    game.bullet_pool = ObjectPool::new(|| Bullet::new(), 100)
    game.explosion_pool = ObjectPool::new(|| Explosion::new(), 20)
    
    game
}
```

### Memory Optimization

```script
// Spatial partitioning for collision detection
struct QuadTree {
    bounds: Rect,
    objects: Vec<GameObject>,
    children: Option<[Box<QuadTree>; 4]>,
    max_objects: i32,
    max_depth: i32,
    depth: i32
}

impl QuadTree {
    fn new(bounds: Rect, max_objects: i32, max_depth: i32, depth: i32) -> QuadTree {
        QuadTree {
            bounds,
            objects: Vec::new(),
            children: None,
            max_objects,
            max_depth,
            depth
        }
    }
    
    fn insert(mut self, obj: GameObject) {
        if !self.bounds.contains(obj.get_bounds()) {
            return  // Object doesn't fit in this quad
        }
        
        if vec_len(self.objects) < self.max_objects || self.depth >= self.max_depth {
            vec_push(self.objects, obj)
            return
        }
        
        // Split if necessary
        if self.children.is_none() {
            self.split()
        }
        
        // Try to insert into children
        if let Some(children) = &mut self.children {
            for child in children {
                child.insert(obj)
            }
        }
    }
    
    fn split(mut self) {
        let half_width = self.bounds.width / 2
        let half_height = self.bounds.height / 2
        
        let children = [
            Box::new(QuadTree::new(
                rect(self.bounds.x, self.bounds.y, half_width, half_height),
                self.max_objects, self.max_depth, self.depth + 1
            )),
            Box::new(QuadTree::new(
                rect(self.bounds.x + half_width, self.bounds.y, half_width, half_height),
                self.max_objects, self.max_depth, self.depth + 1
            )),
            Box::new(QuadTree::new(
                rect(self.bounds.x, self.bounds.y + half_height, half_width, half_height),
                self.max_objects, self.max_depth, self.depth + 1
            )),
            Box::new(QuadTree::new(
                rect(self.bounds.x + half_width, self.bounds.y + half_height, half_width, half_height),
                self.max_objects, self.max_depth, self.depth + 1
            ))
        ]
        
        self.children = Some(children)
    }
    
    fn query(self, bounds: Rect) -> Vec<GameObject> {
        let mut result = Vec::new()
        
        if !self.bounds.intersects(bounds) {
            return result
        }
        
        // Check objects in this quad
        for obj in self.objects {
            if obj.get_bounds().intersects(bounds) {
                vec_push(result, obj)
            }
        }
        
        // Check children
        if let Some(children) = self.children {
            for child in children {
                let child_results = child.query(bounds)
                for obj in child_results {
                    vec_push(result, obj)
                }
            }
        }
        
        result
    }
}
```

## Publishing and Distribution

### Building for Release

```script
// Build configuration
struct BuildConfig {
    target_platform: Platform,
    optimization_level: i32,
    debug_symbols: bool,
    asset_compression: bool,
    obfuscate_code: bool
}

enum Platform {
    Windows,
    MacOS,
    Linux,
    WebAssembly,
    Android,
    iOS
}

fn build_game(config: BuildConfig) -> Result<string, string> {
    print("Building game for " + config.target_platform.to_string())
    
    // Compile with optimizations
    let compile_result = compile_with_options(CompileOptions {
        optimization: config.optimization_level,
        debug_info: config.debug_symbols,
        target: config.target_platform
    })
    
    match compile_result {
        Ok(executable) => {
            // Package assets
            let asset_result = package_assets(config.asset_compression)
            match asset_result {
                Ok(asset_package) => {
                    // Create final distribution
                    create_distribution_package(executable, asset_package)
                },
                Err(error) => Result::err("Asset packaging failed: " + error)
            }
        },
        Err(error) => Result::err("Compilation failed: " + error)
    }
}

// Asset pipeline
fn package_assets(compress: bool) -> Result<AssetPackage, string> {
    let mut package = AssetPackage::new()
    
    // Process sprites
    let sprite_files = find_files("assets/sprites/", "*.png")
    for file in sprite_files {
        let processed = if compress {
            compress_texture(file)
        } else {
            load_file(file)
        }
        
        package.add_asset(file, processed)
    }
    
    // Process audio
    let audio_files = find_files("assets/audio/", "*.wav")
    for file in audio_files {
        let processed = if compress {
            compress_audio(file)
        } else {
            load_file(file)
        }
        
        package.add_asset(file, processed)
    }
    
    Result::ok(package)
}

// Distribution
fn create_installer(game_package: GamePackage) -> Result<string, string> {
    let installer_config = InstallerConfig {
        app_name: "My Awesome Game",
        version: "1.0.0",
        publisher: "My Game Studio",
        install_dir: "MyAwesomeGame",
        create_desktop_shortcut: true,
        create_start_menu: true,
        license_file: "LICENSE.txt"
    }
    
    create_platform_installer(game_package, installer_config)
}
```

---

Congratulations! You now have a comprehensive understanding of game development with Script. This guide covered everything from basic concepts to complete game implementations, performance optimization, and distribution.

## Next Steps

1. **Start Small**: Begin with simple games like Pong or Tetris
2. **Experiment**: Try the code examples and modify them
3. **Join the Community**: Share your games and get feedback
4. **Contribute**: Help improve Script's game development features

## Resources

- **Script Game Library Documentation**: Detailed API reference
- **Community Examples**: More game implementations
- **Performance Guide**: Advanced optimization techniques
- **Asset Creation Tools**: Recommended tools for creating game assets

Happy game development with Script! 🎮

---

*This tutorial is part of the Script programming language documentation. For more information, visit the [Script GitHub repository](https://github.com/moikapy/script).*