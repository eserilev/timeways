-- `/twdev fps`: the frame rate once a second, to measure what a local model on the same GPU
-- costs the game (TESTING.md, "Testing the local model and the frame rate"). A run goes to
-- the desktop as one dev line at its stop. It stops by itself after 15 minutes.

local addonName, ns = ...

local Dev = ns.Dev

local DevFps = {}
ns.DevFps = DevFps

local MAX_SECONDS = 15 * 60
-- The most samples of one line (crates/story/src/dev_fps.rs). A longer run sends the means
-- of equal groups.
local MAX_SENT = 240
local MAX_LABEL = 32

-- The run that samples now: its label, its start, its samples, and the hidden ones.
local run

local function Round(x)
	return math.floor(x + 0.5)
end

-- The value at fraction `p` of a sorted list, between its two nearest items. Nil for an
-- empty list.
function DevFps.Percentile(sorted, p)
	local count = #sorted
	if count == 0 then
		return nil
	end
	local rank = 1 + p * (count - 1)
	local low = math.floor(rank)
	local high = math.min(low + 1, count)
	return sorted[low] + (rank - low) * (sorted[high] - sorted[low])
end

-- The min, the 5th percentile, the median, and the mean, or nil with no samples.
function DevFps.Summary(samples)
	if #samples == 0 then
		return nil
	end
	local sorted, sum = {}, 0
	for i, sample in ipairs(samples) do
		sorted[i] = sample
		sum = sum + sample
	end
	table.sort(sorted)
	return {
		min = sorted[1],
		p5 = DevFps.Percentile(sorted, 0.05),
		median = DevFps.Percentile(sorted, 0.5),
		mean = sum / #samples,
	}
end

-- At most MAX_SENT whole numbers: the samples, or the means of equal groups of them.
function DevFps.Sent(samples)
	local size = math.max(1, math.ceil(#samples / MAX_SENT))
	local sent = {}
	for first = 1, #samples, size do
		local last = math.min(first + size - 1, #samples)
		local sum = 0
		for i = first, last do
			sum = sum + samples[i]
		end
		sent[#sent + 1] = Round(sum / (last - first + 1))
	end
	return sent
end

-- The memory of Timeways in KB, or nil when the game hides it.
local function MemoryKb()
	UpdateAddOnMemoryUsage()
	local kb = GetAddOnMemoryUsage(addonName)
	if issecretvalue(kb) then
		return nil
	end
	return Round(kb)
end

-- The CPU time of Timeways in ms since the login. Nil while the scriptProfile cvar is off.
local function CpuMs()
	local profiling = C_CVar.GetCVar("scriptProfile")
	if issecretvalue(profiling) or profiling ~= "1" then
		return nil
	end
	UpdateAddOnCPUUsage()
	local ms = GetAddOnCPUUsage(addonName)
	if issecretvalue(ms) then
		return nil
	end
	return ms
end

-- The cap of maxFPSBk when the run sat at it, or nil. The game holds the frame rate at this
-- cap while its window is in the background, so such a run measures the cap, not the game.
local function BackgroundCap(summary, samples)
	local value = C_CVar.GetCVar("maxFPSBk")
	if issecretvalue(value) then
		return nil
	end
	local cap = tonumber(value)
	if not cap or cap <= 0 or summary.median < cap - 1 then
		return nil
	end
	for _, sample in ipairs(samples) do
		if sample > cap + 1 then
			return nil
		end
	end
	return cap
end

function DevFps.IsRunning()
	return run ~= nil
end

local function Sample()
	local fps = GetFramerate()
	if issecretvalue(fps) then
		run.hidden = run.hidden + 1
	else
		run.samples[#run.samples + 1] = fps
	end
end

function DevFps.Start(label)
	if run then
		Dev.Say('an FPS run is on. Type "/twdev fps stop" first.')
		return
	end
	run = { label = label, started = time(), samples = {}, hidden = 0, cpu = CpuMs() }
	Sample()
	Dev.Say(string.format('FPS run "%s" started. Type "/twdev fps stop" to end it.', label))
end

local function SayCost(memory, cpu)
	local cost = memory and string.format("Timeways uses %d KB of memory", memory) or "Timeways hides its memory"
	if cpu then
		Dev.Say(string.format("%s, and used %d ms of CPU in the run.", cost, cpu))
	else
		Dev.Say(cost .. ". For its CPU, type /console scriptProfile 1, then /reload.")
	end
end

-- The line of a run. The mark is set here, because a stop after 15 minutes runs outside
-- a command of /twdev.
local function RunLine(stopped, summary, memory, cpu)
	return {
		type = "dev_fps",
		label = stopped.label,
		started = stopped.started,
		ended = time(),
		samples = DevFps.Sent(stopped.samples),
		hidden = stopped.hidden,
		min = Round(summary.min),
		p5 = Round(summary.p5),
		median = Round(summary.median),
		mean = Round(summary.mean),
		memory_kb = memory,
		cpu_ms = cpu,
		dev = true,
	}
end

-- The numbers of the run, or nil when it had no samples.
function DevFps.Stop()
	if not run then
		Dev.Say("no FPS run is on.")
		return
	end
	local stopped = run
	run = nil
	local summary = DevFps.Summary(stopped.samples)
	if not summary then
		Dev.Say("the FPS run has no samples, so nothing was sent.")
		return
	end
	local memory, cpu = MemoryKb(), CpuMs()
	local used = cpu and stopped.cpu and Round(cpu - stopped.cpu)
	ns.Outbox.Add(RunLine(stopped, summary, memory, used))
	ns.Outbox.Flush()
	Dev.Say(
		string.format(
			'FPS run "%s": min %.1f, low 5%% %.1f, median %.1f, mean %.1f, from %d samples (%d hidden).',
			stopped.label,
			summary.min,
			summary.p5,
			summary.median,
			summary.mean,
			#stopped.samples,
			stopped.hidden
		)
	)
	SayCost(memory, used)
	local cap = BackgroundCap(summary, stopped.samples)
	if cap then
		Dev.Say(
			string.format(
				"The frame rate sat at %d, the cap of maxFPSBk while the game window is in the background. "
					.. "Keep the game window in front for a real number.",
				cap
			)
		)
	end
	return {
		samples = #stopped.samples,
		hidden = stopped.hidden,
		min = Round(summary.min),
		p5 = Round(summary.p5),
		median = Round(summary.median),
		mean = Round(summary.mean),
		background_cap = cap,
	}
end

-- A run ends without its line when dev mode turns off: the desktop refuses a dev line then.
local function Tick()
	if not run then
		return
	end
	if not Dev.IsOn() then
		run = nil
		return
	end
	Sample()
	if time() - run.started >= MAX_SECONDS then
		Dev.Say("the FPS run reached 15 minutes, so it stopped.")
		DevFps.Stop()
	end
end

local function PlainLabel(label)
	return #label <= MAX_LABEL and not label:find("[^%w_%-]")
end

Dev.Add("fps", "fps start [label] | stop: sample the frame rate once a second, up to 15 minutes.", function(rest)
	local word, label = rest:match("^(%S*)%s*(.-)$")
	word = word:lower()
	if word == "start" and PlainLabel(label) then
		DevFps.Start(label ~= "" and label or "fps")
	elseif word == "stop" then
		DevFps.Stop()
	else
		Dev.Say("usage: /twdev fps start [label of letters, digits, - and _] | stop")
	end
end)

C_Timer.NewTicker(1, Tick)
