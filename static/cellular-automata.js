/* Cellular Automata on the torn paper background.
 * Three modes, randomly selected each page load:
 *   0 - Game of Life with continuous random seeding
 *   1 - Game of Life seeded from the diagonal tear edge
 *   2 - Glider chaos: grid packed with gliders in random directions
 */
(function() {
    'use strict';
    var canvas = document.getElementById('ca-canvas');
    if (!canvas) return;
    var ctx = canvas.getContext('2d');

    var CELL = 4;   // CSS pixels per cell
    var TPS  = 8;   // ticks per second

    // Cell color: faint blue-gray marks on white paper
    var CR = 0, CG = 0, CB = 30, CA = 11;

    var cols, rows, grid, buf, imgData;
    var mode;

    // ---- Setup ----

    function init() {
        // Fixed size: wide enough for the diagonal to reach the top
        // (900 / tan(8°) ≈ 6405px). Cells are invisible on the dark
        // space background so no clip-path is needed.
        var w = Math.max(document.documentElement.clientWidth, 2000);
        var h = 1200;
        canvas.style.width = w + 'px';
        canvas.style.height = h + 'px';
        cols = Math.ceil(w / CELL);
        rows = Math.ceil(h / CELL);
        canvas.width = cols;
        canvas.height = rows;
        grid = new Uint8Array(cols * rows);
        buf  = new Uint8Array(cols * rows);
        imgData = ctx.createImageData(cols, rows);
        return true;
    }

    function at(x, y) {
        return (x >= 0 && x < cols && y >= 0 && y < rows) ? grid[y * cols + x] : 0;
    }

    // ---- Game of Life step (double-buffered) ----

    function stepGoL() {
        for (var y = 0; y < rows; y++) {
            for (var x = 0; x < cols; x++) {
                var n = at(x-1,y-1)+at(x,y-1)+at(x+1,y-1)+
                        at(x-1,y)            +at(x+1,y)+
                        at(x-1,y+1)+at(x,y+1)+at(x+1,y+1);
                var i = y * cols + x;
                buf[i] = grid[i] ? ((n===2||n===3)?1:0) : (n===3?1:0);
            }
        }
        var t = grid; grid = buf; buf = t;
    }

    // ---- Mode 0: Random seeding + GoL ----

    function initMode0() {
        for (var i = 0; i < grid.length; i++)
            grid[i] = Math.random() < 0.25 ? 1 : 0;
    }

    function tickMode0() {
        for (var i = 0; i < 50; i++)
            grid[Math.floor(Math.random() * grid.length)] = 1;
        stepGoL();
    }

    // ---- Mode 1: Diagonal edge seeding + GoL ----

    var edgeSpread;

    function initMode1() {
        edgeSpread = 4;
    }

    function tickMode1() {
        edgeSpread = Math.min(edgeSpread + 2.0, rows * 0.8);
        var spread = Math.floor(edgeSpread);

        var vw = canvas.clientWidth;
        var leftY  = 900 / CELL;
        var rightY = (900 - vw * Math.tan(8 * Math.PI / 180)) / CELL;
        for (var x = 0; x < cols; x++) {
            var edgeY = Math.floor(leftY + (x / cols) * (rightY - leftY));
            for (var dy = -spread; dy <= 0; dy++) {
                var y = edgeY + dy;
                if (y >= 0 && y < rows && Math.random() < 0.06)
                    grid[y * cols + x] = 1;
            }
        }
        stepGoL();
    }

    // ---- Mode 2: Glider chaos ----
    // Spaceship patterns in all 4 directions, 3 types:
    // Standard glider (5 cells, speed c/4)
    // Lightweight spaceship / LWSS (9 cells, speed c/2)
    // R-pentomino (5 cells, chaotic long-lived methuselah)
    var SHIPS = [
        // -- Standard gliders (4 directions) --
        // SE
        [[0,1],[1,2],[2,0],[2,1],[2,2]],
        // SW
        [[0,1],[1,0],[2,0],[2,1],[2,2]],
        // NE
        [[0,0],[0,1],[0,2],[1,2],[2,1]],
        // NW
        [[0,0],[0,1],[0,2],[1,0],[2,1]],
        // -- LWSS (4 directions) --
        // East
        [[0,1],[0,4],[1,0],[2,0],[2,4],[3,0],[3,1],[3,2],[3,3]],
        // West
        [[0,0],[0,3],[1,4],[2,0],[2,4],[3,1],[3,2],[3,3],[3,4]],
        // South
        [[0,3],[1,0],[1,3],[2,3],[3,3],[4,0],[4,1],[4,2],[4,3]],
        // North
        [[0,0],[0,1],[0,2],[0,3],[1,0],[2,0],[3,0],[3,3],[4,3]],
        // -- R-pentomino (chaotic debris generator) --
        [[0,1],[0,2],[1,0],[1,1],[2,1]]
    ];

    function placeShip(cx, cy) {
        var g = SHIPS[Math.floor(Math.random() * SHIPS.length)];
        for (var i = 0; i < g.length; i++) {
            var x = cx + g[i][1], y = cy + g[i][0];
            if (x >= 0 && x < cols && y >= 0 && y < rows)
                grid[y * cols + x] = 1;
        }
    }

    function initMode2() {
        var spacing = 12;
        for (var gy = 2; gy < rows - 2; gy += spacing) {
            for (var gx = 2; gx < cols - 2; gx += spacing) {
                var jx = gx + Math.floor(Math.random() * 5) - 2;
                var jy = gy + Math.floor(Math.random() * 5) - 2;
                placeShip(jx, jy);
            }
        }
    }

    function tickMode2() {
        stepGoL();
    }

    // ---- Draw ----

    function draw() {
        var d = imgData.data;
        d.fill(0);
        for (var i = 0; i < grid.length; i++) {
            if (grid[i]) {
                var p = i << 2;
                d[p]   = CR;
                d[p+1] = CG;
                d[p+2] = CB;
                d[p+3] = CA;
            }
        }
        ctx.putImageData(imgData, 0, 0);
    }

    // ---- Main loop ----

    if (!init()) return;

    // Cycle: edge(1) → glider(2) → random(0) → repeat
    var ORDER = [1, 2, 0];
    var prev = document.cookie.replace(/(?:^|.*;\s*)ca_seq\s*=\s*(\d).*$/, '$1');
    var seq = prev.length === 1 ? (parseInt(prev, 10) + 1) % 3 : 0;
    document.cookie = 'ca_seq=' + seq + ';path=/;max-age=31536000;SameSite=Lax';
    mode = ORDER[seq];
    var initFns = [initMode0, initMode1, initMode2];
    var tickFns = [tickMode0, tickMode1, tickMode2];

    initFns[mode]();

    var tickMs = 1000 / TPS;
    var lastTick = 0;

    function loop(ts) {
        if (ts - lastTick >= tickMs) {
            tickFns[mode]();
            draw();
            lastTick = ts;
        }
        requestAnimationFrame(loop);
    }
    requestAnimationFrame(loop);
})();
